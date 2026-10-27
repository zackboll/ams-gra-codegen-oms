with Ada.Characters.Handling;
with Ada.Exceptions;
with Ada.Real_Time;
with Ada.Streams;
with Ada.Strings.Fixed;
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
   package F renames AHT.Websocket.Frames;
   CRLF : constant String := ASCII.CR & ASCII.LF;

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
      if S /= No_Socket then Close_Socket (S); end if;
   end Drop;

   procedure Write_All (Self : in out Client; Data : AHT.Bytes.Byte_Array) is
      Deadline : constant Ada.Real_Time.Time :=
        Ada.Real_Time.Clock + Ada.Real_Time.Seconds (2);
      Pos : Positive := Data'First;
      Wire : Stream_Element_Array (1 .. Stream_Element_Offset (Data'Length));
      Last : Stream_Element_Offset;
   begin
      for I in Data'Range loop
         Wire (Stream_Element_Offset (I - Data'First + 1)) := Stream_Element (Data (I));
      end loop;
      while Pos <= Data'Length loop
         if Ada.Real_Time.Clock >= Deadline then
            raise Transport_Error with "write deadline";
         end if;
         if Ready (Self, True, 0.01) then
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

   procedure Reply_Control
     (Self : in out Client; Op : AHT.Websocket.Opcode; Data : AHT.Bytes.Byte_Array)
   is
      Header : F.Frame_Header := (Op => Op, Masked => True, others => <>);
   begin
      --  Control output never waits in Poll and never remains queued. A short
      --  or failed nonblocking send poisons the connection in Poll's handler.
      AHT.Random.Fill (Header.Mask_Key);
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
      Deadline : constant Ada.Real_Time.Time :=
        Ada.Real_Time.Clock + Ada.Real_Time.Seconds (2);
   begin
      if Self.Socket /= No_Socket then
         raise Transport_Error with "already open; close before reopening";
      end if;
      if URL'Length > 2_048 or else URL'Length < 6 or else
        URL (URL'First .. URL'First + 4) /= "ws://"
      then raise Transport_Error with "only loopback ws:// supported; no TLS"; end if;
      for I in URL'Range loop
         if URL (I) < '!' or else URL (I) > '~' or else URL (I) = '#' then
            raise Transport_Error with "invalid URL";
         end if;
      end loop;
      Colon := Index (URL (URL'First + 5 .. URL'Last), ":");
      if Colon = 0 then raise Transport_Error with "explicit port required"; end if;
      declare Host : constant String := URL (URL'First + 5 .. Colon - 1); begin
         if Host /= "127.0.0.1" and then Host /= "localhost" then
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
      Create_Socket (Self.Socket);
      Set_Close_On_Exec (Self.Socket, True, Exec_OK);
      if not Exec_OK then raise Transport_Error with "close-on-exec failed"; end if;
      Connect_Socket (Self.Socket, (Family_Inet, Inet_Addr ("127.0.0.1"), Port), 2.0,
                      Status => Status);
      if Status /= Completed then raise Transport_Error with "connect deadline"; end if;
      Control_Socket (Self.Socket, Nonblocking);
      AHT.Random.Fill (Nonce);
      declare
         Key : constant String := AHT.Bytes.Base64_Encode (Nonce);
         Request : constant String := OMS_WS_Handshake.Build_Client_Request
           (URL (URL'First + 5 .. Colon - 1), Natural (Port),
            (if Slash > URL'Last then "/" else URL (Slash .. URL'Last)), Key);
      begin
         Write_All (Self, AHT.Bytes.From_String
           (Request (Request'First .. Request'Last - 2) & "Sec-WebSocket-Protocol: owp" & CRLF & CRLF));
         loop
            if Length (Response) >= 16_384 then raise Transport_Error with "excessive headers"; end if;
            if Ada.Real_Time.Clock >= Deadline then raise Transport_Error with "upgrade deadline"; end if;
            if Ready (Self, False, 0.01) then
               Receive_Socket (Self.Socket, One, Last);
               if Last = 0 then raise Transport_Error with "EOF in upgrade"; end if;
               Append (Response, Character'Val (One (1)));
               exit when Length (Response) >= 4 and then
                 Slice (Response, Length (Response) - 3, Length (Response)) = CRLF & CRLF;
            end if;
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
      Self.Upgraded := True;
   exception
      when E : others =>
         Drop (Self);
         raise Transport_Error with Ada.Exceptions.Exception_Message (E);
   end Open;

   overriding procedure Send (Self : in out Client; Frame : String) is
      Message : String renames Frame;
   begin
      if not Self.Upgraded then raise Transport_Error with "not open"; end if;
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
   begin
      Message := Null_Unbounded_String;
      Status := Idle;
      if not Self.Upgraded then Status := Peer_Closed; return; end if;
      --  At most one nonblocking 4 KiB read and one frame decode per call.
      if Self.Input.Length >= 2 and then
        (Self.Input.Element (2) and 16#80#) /= 0
      then raise Transport_Error with "masked server frame"; end if;
      F.Decode (Self.Input.To_Array, Header, Payload, Last,
                Allowance => Max_Frame_Bytes - Self.Fragments.Length,
                Fragment => Self.Fragment);
      if Last = 0 then
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
            --  Echo close with the same nonblocking all-or-error policy.
            if not Self.Close_Sent then
               Reply_Control (Self, AHT.Websocket.Close_Op, Payload.To_Array);
            end if;
            Drop (Self); Status := Peer_Closed;
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
        Ada.Real_Time.Clock + Ada.Real_Time.Milliseconds (250);
      Status : Receive_Status;
      Message : Unbounded_String;
   begin
      if Self.Upgraded then
         begin Send_Control (Self, AHT.Websocket.Close_Op, [16#03#, 16#E8#]);
         exception when E : others =>
            Drop (Self);
            raise Transport_Error with Ada.Exceptions.Exception_Message (E);
         end;
         Self.Close_Sent := True;
         while Self.Upgraded and then Ada.Real_Time.Clock < Deadline loop
            Poll (Self, Status, Message);
            if Self.Upgraded then delay 0.001; end if;
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