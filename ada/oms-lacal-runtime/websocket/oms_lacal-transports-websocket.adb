with Ada.Characters.Handling;
with Ada.Exceptions;
with Ada.Real_Time;
with Ada.Streams;
with Ada.Strings.Fixed;
with Ada.Unchecked_Deallocation;
with AHT.URL;
with AHT.Random;
with AHT.Websocket.UTF8;
with OMS_WS_Handshake;

package body OMS_LACAL.Transports.WebSocket is
   use GNAT.Sockets;
   use Ada.Streams;
   use Ada.Strings.Unbounded;
   use type Ada.Real_Time.Time;
   use type AHT.Websocket.Opcode;
   use type AHT.Websocket.Frames.Fragment_State;
   use type AHT.Websocket.Frames.Inbound_Close_Status;
   use type OMS_WS_Handshake.Status_Kind;
   use type AHT.Bytes.Octet;
   use type AHT.Transport.TLS.Try_Read_Status;
   use type AHT.Transport.TLS.Try_Write_Status;
   use type AHT.Transport.TLS.Try_Flush_Status;
   procedure Free_TLS is new Ada.Unchecked_Deallocation
     (AHT.Transport.TLS.TLS_Transport, TLS_Access);
   package F renames AHT.Websocket.Frames;
   CRLF : constant String := ASCII.CR & ASCII.LF;

   procedure Set_Absolute_Deadline
     (Self : in out Client; Deadline : Ada.Real_Time.Time) is
   begin
      Self.Absolute_Deadline := Deadline;
      Self.Has_Deadline := True;
   end Set_Absolute_Deadline;

   function Bound (Self : Client; Operation : Ada.Real_Time.Time)
                   return Ada.Real_Time.Time is
     (if Self.Has_Deadline and then Self.Absolute_Deadline < Operation
      then Self.Absolute_Deadline else Operation);

   function Remaining (Deadline : Ada.Real_Time.Time) return Duration is
      Now : constant Ada.Real_Time.Time := Ada.Real_Time.Clock;
   begin
      if Now >= Deadline then
         raise Transport_Error with "operation deadline";
      end if;
      return Ada.Real_Time.To_Duration (Deadline - Now);
   end Remaining;

   procedure Check_Caller (Self : Client) is
   begin
      if Self.Has_Deadline and then Ada.Real_Time.Clock >= Self.Absolute_Deadline then
         raise Transport_Error with "caller deadline";
      end if;
   end Check_Caller;

   function Ready (Self : Client; Write : Boolean; Wait : Duration) return Boolean is
      R, W : Socket_Set_Type;
      Status : Selector_Status;
   begin
      Empty (R); Empty (W);
      if Write then Set (W, Self.Socket); else Set (R, Self.Socket); end if;
      Check_Selector (Null_Selector, R, W, Status, Wait);
      return Status = Completed;
   end Ready;

   procedure Drop (Self : in out Client) is
      S : constant Socket_Type := Self.Socket;
   begin
      Self.Socket := No_Socket;
      Self.Upgraded := False;
      Self.Close_Sent := False;
      Self.Input.Clear;
      Self.Fragments.Clear;
      Self.Fragment := F.No_Message;
      Self.Output.Clear;
      Self.Output_Pos := 1;
      Self.Peer_Closing := False;
      if Self.TLS /= null then
         Self.TLS.Abortive_Close;
         Free_TLS (Self.TLS);
      end if;
      if S /= No_Socket then Close_Socket (S); end if;
   end Drop;

   procedure Write_All (Self : in out Client; Data : AHT.Bytes.Byte_Array) is
      Deadline : constant Ada.Real_Time.Time :=
         Bound (Self, Ada.Real_Time.Clock + Ada.Real_Time.Seconds (2));
      Pos : Positive := Data'First;
      Wire : Stream_Element_Array (1 .. Stream_Element_Offset (Data'Length));
      Last : Stream_Element_Offset;
   begin
      Check_Caller (Self);
      if Self.TLS /= null then
         --  Send is explicitly allowed to block. Drain queued controls before
         --  this publication, and fail closed on any uncertain/incomplete write.
         --  One compound budget for queued controls AND the new frame. AHT's
         --  setter stores an absolute bound; do not reset it between writes.
         Self.TLS.Set_Write_Deadline (Remaining (Deadline));
         if not Self.Output.Is_Empty then
            Self.TLS.Write (Self.Output.To_Array
              (Self.Output_Pos, Self.Output.Length));
            Self.Output.Clear;
            Self.Output_Pos := 1;
         end if;
         Check_Caller (Self);
         Self.TLS.Write (Data);
         Check_Caller (Self);
         return;
      end if;
      for I in Data'Range loop
         Wire (Stream_Element_Offset (I - Data'First + 1)) := Stream_Element (Data (I));
      end loop;
      while Pos <= Data'Length loop
         if Ada.Real_Time.Clock >= Deadline then
            raise Transport_Error with "write deadline";
         end if;
         if Ready (Self, True, Duration'Min (0.01, Remaining (Deadline))) then
            begin
               Send_Socket (Self.Socket, Wire (Stream_Element_Offset (Pos) .. Wire'Last), Last);
               if Last < Stream_Element_Offset (Pos) then
                  raise Transport_Error with "zero write";
               end if;
               Pos := Natural (Last) + 1;
            exception
               when E : Socket_Error =>
                  if Resolve_Exception (E) /= Resource_Temporarily_Unavailable then raise; end if;
            end;
         end if;
      end loop;
      Check_Caller (Self);
   exception
      when others => Drop (Self); raise;
   end Write_All;

   procedure Send_Control
     (Self : in out Client; Op : AHT.Websocket.Opcode; Data : AHT.Bytes.Byte_Array)
   is
      Header : F.Frame_Header := (Op => Op, Masked => True, others => <>);
   begin
      AHT.Random.Fill (Header.Mask_Key);
      Write_All (Self, F.Encode (Header, Data));
   end Send_Control;

   --  Exactly one flush and at most one engine write per call. Needs_Read
   --  leaves plaintext untouched: Poll takes its one receive step afterwards,
   --  never recursively invoking this procedure or spinning on readiness.
   procedure Progress_Output (Self : in out Client) is
      Flush : AHT.Transport.TLS.Try_Flush_Status;
      Result : AHT.Transport.TLS.Try_Write_Status;
      Count : Natural;
   begin
      Self.TLS.Try_Flush (Flush);
      if Flush = AHT.Transport.TLS.Would_Block or else Self.Output.Is_Empty then
         return;
      end if;
      Self.TLS.Try_Write
        (Self.Output.To_Array (Self.Output_Pos, Self.Output.Length), Count, Result);
      case Result is
         when AHT.Transport.TLS.Accepted =>
            Self.Output_Pos := Self.Output_Pos + Count;
            if Self.Output_Pos > Self.Output.Length then
               Self.Output.Clear;
               Self.Output_Pos := 1;
            end if;
         when AHT.Transport.TLS.Would_Block | AHT.Transport.TLS.Needs_Read =>
            if Count /= 0 then raise Transport_Error with "invalid TLS acceptance"; end if;
      end case;
   end Progress_Output;

   procedure Reply_Control
     (Self : in out Client; Op : AHT.Websocket.Opcode; Data : AHT.Bytes.Byte_Array)
   is
      Header : F.Frame_Header := (Op => Op, Masked => True, others => <>);
   begin
      Check_Caller (Self);
      --  Plaintext keeps its all-or-error send policy; TLS retains one bounded
      --  frame and progresses it on subsequent Poll calls without waiting.
      AHT.Random.Fill (Header.Mask_Key);
      if Self.TLS /= null then
         if not Self.Output.Is_Empty then
            raise Transport_Error with "control output queue full";
         end if;
         Self.Output.Set (F.Encode (Header, Data));
         Self.Output_Pos := 1;
         return;
      end if;
      declare
         Bytes : constant AHT.Bytes.Byte_Array := F.Encode (Header, Data);
         Wire : Stream_Element_Array (1 .. Stream_Element_Offset (Bytes'Length));
         Sent : Stream_Element_Offset;
      begin
         for I in Bytes'Range loop
            Wire (Stream_Element_Offset (I)) := Stream_Element (Bytes (I));
         end loop;
         Send_Socket (Self.Socket, Wire, Sent);
         if Sent /= Wire'Last then
            raise Transport_Error with "uncertain control write";
         end if;
      end;
   end Reply_Control;

   procedure Open (Self : in out Client; URL : String) is
   begin
      Open (Self, URL, CA_Path => "");
   end Open;

   procedure Open
     (Self : in out Client; URL : String; CA_Path : String; CA_PEM : String := "") is
      use Ada.Strings.Fixed;
      use Ada.Characters.Handling;
      Colon, Slash : Natural := 0;
      Port : Port_Type;
      Status : Selector_Status;
      Exec_OK : Boolean;
      Nonblocking : Request_Type := (Non_Blocking_IO, True);
      Nonce : AHT.Bytes.Byte_Array (1 .. 16);
      Response : Unbounded_String;
      One : Stream_Element_Array (1 .. 1);
      Last : Stream_Element_Offset;
      Secure : constant Boolean := URL'Length >= 6 and then
        URL (URL'First .. URL'First + 5) = "wss://";
      Prefix : constant Natural := (if Secure then 6 else 5);
      TLS_One : AHT.Bytes.Byte_Array (1 .. 1);
      TLS_Last : Natural;
      Deadline : Ada.Real_Time.Time :=
         Bound (Self, Ada.Real_Time.Clock + Ada.Real_Time.Seconds (2));
   begin
      Check_Caller (Self);
      if Self.Socket /= No_Socket or else Self.TLS /= null then
         raise Transport_Error with "already open; close before reopening";
      end if;
      if URL'Length > 2_048 or else URL'Length < 6 or else
        (not Secure and then URL (URL'First .. URL'First + 4) /= "ws://")
      then raise Transport_Error with "expected ws:// or wss://"; end if;
      for I in URL'Range loop
         if URL (I) < '!' or else URL (I) > '~' or else URL (I) = '#' then
            raise Transport_Error with "invalid URL";
         end if;
      end loop;
      Colon := Index (URL (URL'First + Prefix .. URL'Last), ":");
      if Colon = 0 then raise Transport_Error with "explicit port required"; end if;
      declare Host : constant String := URL (URL'First + Prefix .. Colon - 1); begin
         if Host'Length = 0 or else Index (Host, "@") /= 0 then
            raise Transport_Error with "invalid host";
         end if;
         if not Secure and then Host /= "127.0.0.1" and then Host /= "localhost" then
            raise Transport_Error with "plaintext restricted to loopback";
         end if;
      end;
      Slash := Index (URL (Colon + 1 .. URL'Last), "/");
      if Slash = 0 then Slash := URL'Last + 1; end if;
      if Colon + 1 >= Slash then raise Transport_Error with "missing port"; end if;
      for C of URL (Colon + 1 .. Slash - 1) loop
         if C not in '0' .. '9' then raise Transport_Error with "invalid port"; end if;
      end loop;
      Port := Port_Type'Value (URL (Colon + 1 .. Slash - 1));
      if Port = 0 then raise Transport_Error with "zero port"; end if;
      if Secure then
         Self.TLS := new AHT.Transport.TLS.TLS_Transport;
         declare
            --  AHT's three durations are not a cumulative timeout. Reserve
            --  rounding slack and divide the remaining budget, never grant it
            --  three times. HS_Bound starts at AHT Connect entry in both pinned
            --  bodies, making the actual bound at least as conservative.
            Budget : Duration := 2.0;
         begin
            if Self.Has_Deadline then
               Budget := Duration'Min
                 (2.0, (Remaining (Self.Absolute_Deadline) - 0.002) / 3);
               if Budget <= 0.0 then
                  raise Transport_Error with "insufficient setup deadline";
               end if;
            end if;
            Self.TLS.Connect
              (Host => URL (URL'First + Prefix .. Colon - 1),
               Port => AHT.URL.Port_Number (Port), CA_Path => CA_Path, CA_PEM => CA_PEM,
               Resolve_Timeout => Budget, Connect_Timeout => Budget, Handshake_Deadline => Budget);
         end;
         Check_Caller (Self);
         Deadline := Bound (Self, Ada.Real_Time.Clock + Ada.Real_Time.Seconds (2));
      else
         if CA_Path /= "" or else CA_PEM /= "" then
            raise Transport_Error with "CA configuration requires wss://";
         end if;
         Create_Socket (Self.Socket);
         Set_Close_On_Exec (Self.Socket, True, Exec_OK);
         if not Exec_OK then raise Transport_Error with "close-on-exec failed"; end if;
         Connect_Socket (Self.Socket, (Family_Inet, Inet_Addr ("127.0.0.1"), Port),
                         (if Self.Has_Deadline then Remaining (Deadline) else 2.0),
                         Status => Status);
         if Status /= Completed then raise Transport_Error with "connect deadline"; end if;
         Control_Socket (Self.Socket, Nonblocking);
      end if;
      Check_Caller (Self);
      AHT.Random.Fill (Nonce);
      declare
         Key : constant String := AHT.Bytes.Base64_Encode (Nonce);
         Request : constant String := OMS_WS_Handshake.Build_Client_Request
           (URL (URL'First + Prefix .. Colon - 1), Natural (Port),
            (if Slash > URL'Last then "/" else URL (Slash .. URL'Last)), Key);
      begin
         Write_All (Self, AHT.Bytes.From_String
           (Request (Request'First .. Request'Last - 2) & "Sec-WebSocket-Protocol: owp" & CRLF & CRLF));
         loop
            if Length (Response) >= 16_384 then raise Transport_Error with "excessive headers"; end if;
            if Ada.Real_Time.Clock >= Deadline then raise Transport_Error with "upgrade deadline"; end if;
            if Secure then
               Self.TLS.Set_Read_Deadline
                 (Remaining (Deadline));
               Self.TLS.Read (TLS_One, TLS_Last);
               if TLS_Last = 0 then raise Transport_Error with "EOF in upgrade"; end if;
               Append (Response, Character'Val (TLS_One (1)));
            elsif Ready (Self, False, Duration'Min (0.01, Remaining (Deadline))) then
               Receive_Socket (Self.Socket, One, Last);
               if Last = 0 then raise Transport_Error with "EOF in upgrade"; end if;
               Append (Response, Character'Val (One (1)));
            end if;
            exit when Length (Response) >= 4 and then
              Slice (Response, Length (Response) - 3, Length (Response)) = CRLF & CRLF;
         end loop;
         declare
            S : constant String := To_String (Response);
            Start : Positive := 1;
            Finish, Separator : Natural;
            Have_Protocol : Boolean := False;
            Seen_Upgrade, Seen_Connection, Seen_Accept : Boolean := False;
         begin
            if S'Length < 13 or else S (1 .. 13) /= "HTTP/1.1 101 " then
               raise Transport_Error with "expected HTTP/1.1 101";
            end if;
            Start := Index (S, CRLF) + 2;
            while Start < S'Last - 1 loop
               Finish := Index (S (Start .. S'Last), CRLF);
               Separator := Index (S (Start .. Finish - 1), ":");
               if Separator <= Start then raise Transport_Error with "malformed header"; end if;
               for C of S (Start .. Separator - 1) loop
                  if not (C in 'A' .. 'Z' | 'a' .. 'z' | '0' .. '9' | '-') then
                     raise Transport_Error with "invalid header name";
                  end if;
               end loop;
               for C of S (Separator + 1 .. Finish - 1) loop
                  if (C < ' ' and then C /= ASCII.HT) or else C = ASCII.DEL then
                     raise Transport_Error with "invalid header value";
                  end if;
               end loop;
               declare
                  Name : constant String := To_Lower (S (Start .. Separator - 1));
                  Value : constant String := Trim (S (Separator + 1 .. Finish - 1), Ada.Strings.Both);
               begin
                  if Name = "sec-websocket-protocol" then
                     if Have_Protocol or else Value /= "owp" then
                        raise Transport_Error with "required negotiated owp";
                     end if;
                     Have_Protocol := True;
                  elsif Name = "sec-websocket-extensions" then
                     raise Transport_Error with "extensions unsupported";
                  elsif Name = "upgrade" then
                     if Seen_Upgrade then raise Transport_Error with "duplicate upgrade"; end if;
                     Seen_Upgrade := True;
                  elsif Name = "connection" then
                     if Seen_Connection then raise Transport_Error with "duplicate connection"; end if;
                     Seen_Connection := True;
                  elsif Name = "sec-websocket-accept" then
                     if Seen_Accept then raise Transport_Error with "duplicate accept"; end if;
                     Seen_Accept := True;
                  end if;
               end;
               Start := Finish + 2;
            end loop;
            if not Have_Protocol then raise Transport_Error with "missing negotiated owp"; end if;
            if OMS_WS_Handshake.Validate_Server_Response
              (S, OMS_WS_Handshake.Compute_Accept (Key)).Kind /= OMS_WS_Handshake.OK
            then raise Transport_Error with "invalid upgrade response"; end if;
         end;
      end;
      Check_Caller (Self);
      Self.Upgraded := True;
   exception
      when E : others =>
         Drop (Self);
         raise Transport_Error with Ada.Exceptions.Exception_Message (E);
   end Open;

   overriding procedure Send (Self : in out Client; Frame : String) is
      Message : String renames Frame;
   begin
      if not Self.Upgraded or else Self.Peer_Closing then
         raise Transport_Error with "not open";
      end if;
      if Message'Length > Max_Frame_Bytes or else not AHT.Websocket.UTF8.Is_Valid
        (AHT.Bytes.From_String (Message))
      then raise Transport_Error with "invalid text/size before publication"; end if;
      begin
         Send_Control (Self, AHT.Websocket.Text, AHT.Bytes.From_String (Message));
      exception when E : others =>
         Drop (Self);
         raise Transport_Error with Ada.Exceptions.Exception_Message (E);
      end;
   end Send;

   overriding procedure Poll
     (Self : in out Client; Status : out Receive_Status; Frame : out Text)
   is
      Message : Text renames Frame;
      Header : F.Frame_Header;
      Payload : AHT.Bytes.Buffer;
      Last : Natural;
      Chunk : Stream_Element_Array (1 .. 4_096);
      Got : Stream_Element_Offset;
      TLS_Chunk : AHT.Bytes.Byte_Array (1 .. 4_096);
      TLS_Last : Natural := 0;
      TLS_Status : AHT.Transport.TLS.Try_Read_Status;
      Paused : Boolean := False;
      procedure Read_TLS is
      begin
         Self.TLS.Try_Read (TLS_Chunk, TLS_Last, TLS_Status);
         if TLS_Status = AHT.Transport.TLS.Peer_Closed then
            if Self.Peer_Closing and then
              (not Self.Output.Is_Empty or else Self.TLS.Pending_Output > 0)
            then
               --  Authenticated EOF does not discard an unsent WS Close reply.
               return;
            end if;
            Drop (Self);
            Status := Peer_Closed;
         elsif TLS_Status = AHT.Transport.TLS.Data_Ready then
            if Self.Input.Length + TLS_Last > Max_Frame_Bytes + 4_110 then
               raise Transport_Error with "input limit";
            end if;
            Self.Input.Append (TLS_Chunk (1 .. TLS_Last));
         end if;
         if Self.TLS /= null and then Self.TLS.Pending_Output > 65_536 then
            raise Transport_Error with "TLS control output limit";
         end if;
      end Read_TLS;
   begin
      Message := Null_Unbounded_String;
      Status := Idle;
      if not Self.Upgraded then Status := Peer_Closed; return; end if;
      if Self.TLS /= null then
         Progress_Output (Self);
         Paused := not Self.Output.Is_Empty or else Self.TLS.Pending_Output > 0;
         if Self.Peer_Closing and then not Paused then
            Drop (Self); Status := Peer_Closed; return;
         end if;
         --  Receive even under backpressure (including Needs_Read), but
         --  bounded storage, one receive step, and no additional frame/control
         --  consumption until the previous response is handed off in order.
         if Paused then
            if Self.Input.Length <= Max_Frame_Bytes + 14 then Read_TLS; end if;
            return;
         end if;
      end if;
      --  At most one nonblocking 4 KiB read and one frame decode per call.
      if Self.Input.Length >= 2 and then
        (Self.Input.Element (2) and 16#80#) /= 0
      then raise Transport_Error with "masked server frame"; end if;
      F.Decode (Self.Input.To_Array, Header, Payload, Last,
                Allowance => Max_Frame_Bytes - Self.Fragments.Length,
                Fragment => Self.Fragment);
      if Last = 0 then
         if Self.TLS /= null then
            Read_TLS;
            if not Self.Upgraded then return; end if;
            F.Decode (Self.Input.To_Array, Header, Payload, Last,
                      Allowance => Max_Frame_Bytes - Self.Fragments.Length,
                      Fragment => Self.Fragment);
            if Last = 0 then return; end if;
         else
            if not Ready (Self, False, 0.0) then return; end if;
            begin
               Receive_Socket (Self.Socket, Chunk, Got);
            exception when E : Socket_Error =>
               if Resolve_Exception (E) = Resource_Temporarily_Unavailable then return; end if;
               raise;
            end;
            if Got = 0 then Drop (Self); Status := Peer_Closed; return; end if;
            if Self.Input.Length + Natural (Got) > Max_Frame_Bytes + 4_110 then
               raise Transport_Error with "input limit";
            end if;
            for I in 1 .. Got loop Self.Input.Append (AHT.Bytes.Octet (Chunk (I))); end loop;
            if Self.Input.Length >= 2 and then
              (Self.Input.Element (2) and 16#80#) /= 0
            then raise Transport_Error with "masked server frame"; end if;
            F.Decode (Self.Input.To_Array, Header, Payload, Last,
                      Allowance => Max_Frame_Bytes - Self.Fragments.Length,
                      Fragment => Self.Fragment);
            if Last = 0 then return; end if;
         end if;
      end if;
      if Header.Masked then raise Transport_Error with "masked server frame"; end if;
      Self.Input := Self.Input.Slice (Last + 1, Self.Input.Length);
      case Header.Op is
         when AHT.Websocket.Binary => raise Transport_Error with "binary unsupported";
         when AHT.Websocket.Ping_Op =>
            if Self.Close_Sent then return; end if;
            Reply_Control (Self, AHT.Websocket.Pong_Op, Payload.To_Array);
         when AHT.Websocket.Pong_Op => null;
         when AHT.Websocket.Close_Op =>
            if F.Classify_Inbound_Close (Payload) /= F.Accept_Close then
               raise Transport_Error with "invalid close";
            end if;
            --  Retain the TLS session until its Close echo is flushed.
            if not Self.Close_Sent then
               Reply_Control (Self, AHT.Websocket.Close_Op, Payload.To_Array);
            end if;
            if Self.TLS /= null then
               Self.Peer_Closing := True;
            else
               Drop (Self); Status := Peer_Closed;
            end if;
         when AHT.Websocket.Text | AHT.Websocket.Continuation =>
            Self.Fragments.Append (Payload);
            if Header.FIN then
               if not AHT.Websocket.UTF8.Is_Valid (Self.Fragments) then
                  raise Transport_Error with "invalid UTF-8";
               end if;
               Message := To_Unbounded_String (Self.Fragments.To_String);
               Self.Fragments.Clear;
               Self.Fragment := F.No_Message;
               Status := Frame_Ready;
            else Self.Fragment := F.In_Message; end if;
      end case;
   exception
      when E : others =>
         Drop (Self);
         raise Transport_Error with Ada.Exceptions.Exception_Message (E);
   end Poll;

   overriding procedure Close (Self : in out Client) is
      Deadline : constant Ada.Real_Time.Time :=
         Bound (Self, Ada.Real_Time.Clock + Ada.Real_Time.Milliseconds (250));
      Status : Receive_Status;
      Message : Unbounded_String;
   begin
      if Self.Has_Deadline and then Ada.Real_Time.Clock >= Self.Absolute_Deadline then
         Drop (Self);
         return;
      end if;
      if Self.Upgraded then
         if not Self.Peer_Closing then
            begin Send_Control (Self, AHT.Websocket.Close_Op, [16#03#, 16#E8#]);
            exception when E : others =>
               Drop (Self);
               raise Transport_Error with Ada.Exceptions.Exception_Message (E);
            end;
            Self.Close_Sent := True;
         end if;
         while Self.Upgraded and then Ada.Real_Time.Clock < Deadline loop
            Poll (Self, Status, Message);
            if Self.Upgraded and then Ada.Real_Time.Clock < Deadline then
               delay until Bound (Self, Ada.Real_Time.Clock + Ada.Real_Time.Milliseconds (1));
            end if;
         end loop;
      end if;
      Drop (Self);
   end Close;

   function Protocol (Self : Client) return String is
     (if Self.Upgraded then "owp" else "");

   overriding procedure Finalize (Self : in out Client) is
   begin
      Drop (Self);
   exception when others => null;
   end Finalize;
end OMS_LACAL.Transports.WebSocket;