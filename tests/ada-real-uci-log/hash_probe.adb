with Ada.Text_IO;
with Programs.Oam;
with Service_Codec;

procedure Hash_Probe is
   Accepted, Rejected : Natural := 0;
begin
   for Octets in 16 .. 33 loop
      if Octets in 16 | 31 | 32 | 33 then
         declare
            Hex : constant String (1 .. Octets * 2) := (others => '0');
         begin
            declare
               Item : constant Programs.Oam.BinaryProbe :=
                 Service_Codec.Decode_BinaryProbe
                   ("{""Data"":""0080FF"",""SHA_2_Hash"":""" & Hex & """}");
               pragma Unreferenced (Item);
            begin
               if Octets /= 32 then
                  raise Program_Error with "non-32-octet hash accepted";
               end if;
               Accepted := Accepted + 1;
            end;
         exception
            when Service_Codec.Codec_Error =>
               if Octets = 32 then
                  raise Program_Error with "32-octet hash rejected";
               end if;
               Rejected := Rejected + 1;
         end;
      end if;
   end loop;
   if Accepted /= 1 or else Rejected /= 3 then
      raise Program_Error with "incomplete hash probe";
   end if;
   Ada.Text_IO.Put_Line ("TASK102_HASH_OCTETS_COMPLETE accepted=1 rejected=3");
end Hash_Probe;