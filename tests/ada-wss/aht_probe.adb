with Ada.Command_Line;
with Ada.Exceptions;
with Ada.Real_Time; use Ada.Real_Time;
with Ada.Text_IO; use Ada.Text_IO;
with AHT.Bytes;
with AHT.Transport.TLS;
with AHT.URL;

--  Downstream consumer: no private AHT state and no insecure connection.
procedure AHT_Probe is
   use AHT.Transport.TLS;
   T : TLS_Transport;
   B : AHT.Bytes.Byte_Array (1 .. 4_096);
   Data : AHT.Bytes.Byte_Array (1 .. 32_768);
   Last, Count, Total, Pending : Natural := 0;
   RS : Try_Read_Status;
   WS : Try_Write_Status;
   FS : Try_Flush_Status;
   Start, Deadline : Time;
   Mode : constant String := Ada.Command_Line.Argument (4);
   procedure Prompt (Since : Time) is
   begin
      if Clock - Since > Milliseconds (200) then
         raise Program_Error with "nonblocking call exceeded 200ms";
      end if;
   end Prompt;
begin
   T.Connect
     (Host => Ada.Command_Line.Argument (1),
      Port => AHT.URL.Port_Number'Value (Ada.Command_Line.Argument (2)),
      CA_Path => Ada.Command_Line.Argument (3),
      Connect_Timeout => 2.0, Handshake_Deadline => 2.0,
      Resolve_Timeout => 2.0);
   Put_Line ("VERIFIED_TLS_CONNECTED");
   if Mode = "idle" then
      for I in 1 .. 100 loop
         Start := Clock;
         T.Try_Read (B, Last, RS);
         Prompt (Start);
         if Last /= 0 or else RS /= Would_Block then
            raise Program_Error with "idle read was not Would_Block";
         end if;
         T.Try_Flush (FS);
      end loop;
      Put_Line ("IDLE_READS_COMPLETE 100");
   elsif Mode = "deaf" or else Mode = "sink" then
      Deadline := Clock + Seconds (10);
      loop
         for I in Data'Range loop
            Data (I) := AHT.Bytes.Octet ((Total + I - 1) mod 251);
         end loop;
         Start := Clock;
         T.Try_Write (Data, Count, WS);
         Prompt (Start);
         case WS is
            when Accepted =>
               if Count not in 1 .. 16_384 then
                  raise Program_Error with "incorrect plaintext acceptance";
               end if;
               Total := Total + Count;
            when Would_Block =>
               if Count /= 0 or else T.Pending_Output = 0 then
                  raise Program_Error with "lost pending output";
               end if;
               exit;
            when Needs_Read =>
               if Count /= 0 then raise Program_Error; end if;
               --  One receive step, no recursive write or retry loop.
               T.Try_Read (B, Last, RS);
               delay 0.001;
         end case;
         if Clock > Deadline then raise Program_Error with "no backpressure"; end if;
      end loop;
      Pending := T.Pending_Output;
      Put_Line ("BACKPRESSURE accepted=" & Total'Image & " pending=" & Pending'Image);
      for I in 1 .. 20 loop
         Start := Clock;
         T.Try_Flush (FS);
         Prompt (Start);
         if FS /= Would_Block or else T.Pending_Output /= Pending then
            raise Program_Error with "pending ciphertext not retained";
         end if;
      end loop;
      if Mode = "sink" then
         Put_Line ("RESUME_SINK");
         Deadline := Clock + Seconds (10);
         loop
            Start := Clock;
            T.Try_Flush (FS);
            Prompt (Start);
            exit when FS = Flushed;
            if Clock > Deadline then raise Program_Error with "flush timeout"; end if;
            delay 0.001;
         end loop;
         if T.Pending_Output /= 0 then raise Program_Error; end if;
         Put_Line ("FLUSH_COMPLETE accepted=" & Total'Image);
         --  Socket acceptance is not peer receipt. Wait for its accounting
         --  acknowledgement before deliberately truncating the TLS session.
         loop
            T.Try_Read (B, Last, RS);
            exit when RS = Data_Ready;
            if Clock > Deadline then raise Program_Error with "sink ack timeout"; end if;
            delay 0.001;
         end loop;
      end if;
   elsif Mode = "generated" then
      Put_Line ("AWAIT_TLS_CONTROL");
      Deadline := Clock + Seconds (5);
      loop
         Start := Clock;
         T.Try_Read (B, Last, RS);
         Prompt (Start);
         if RS /= Would_Block or else Last /= 0 then raise Program_Error; end if;
         exit when T.Pending_Output > 0;
         if Clock > Deadline then raise Program_Error with "no TLS control output"; end if;
         delay 0.001;
      end loop;
      Put_Line ("TLS_GENERATED_OUTPUT pending=" & T.Pending_Output'Image);
      Start := Clock;
      T.Try_Flush (FS);
      Prompt (Start);
      if FS /= Flushed or else T.Pending_Output /= 0 then raise Program_Error; end if;
      Put_Line ("TLS_CONTROL_FLUSH_COMPLETE plaintext=0");
      loop
         begin
            T.Try_Read (B, Last, RS);
            exit when RS = Peer_Closed;
         exception
            when TLS_Error => exit;
         end;
         if Clock > Deadline then raise Program_Error with "server did not react"; end if;
         delay 0.001;
      end loop;
   else
      Deadline := Clock + Seconds (5);
      loop
         Start := Clock;
         T.Try_Read (B, Last, RS);
         Prompt (Start);
         exit when RS /= Would_Block;
         if Clock > Deadline then raise Program_Error with "read timeout"; end if;
         delay 0.001;
      end loop;
      Put_Line ("READ_RESULT " & RS'Image & Last'Image);
   end if;
   Start := Clock;
   T.Abortive_Close;
   T.Abortive_Close;
   Prompt (Start);
   if T.Is_Connected or else T.Pending_Output /= 0 then raise Program_Error; end if;
   Put_Line ("AHT_CONSUMER_COMPLETE");
exception
   when E : others =>
      T.Abortive_Close;
      Put_Line ("AHT_CONSUMER_FAILED " & Ada.Exceptions.Exception_Information (E));
      Ada.Command_Line.Set_Exit_Status (1);
end AHT_Probe;