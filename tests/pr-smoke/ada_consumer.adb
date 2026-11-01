with Ada.Text_IO;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with Programs.Oam; use Programs.Oam;
with Service_Codec; use Service_Codec;
with Service_API;

procedure Ada_Consumer is
   Text : constant String :=
     "{""Enabled"":true,""Level"":-7,""Name"":""smoke"",""Signal"":""5G"","
     & """Label"":""checked label"",""Details"":{""Text"":""nested""},"
     & """begin"":""wire name""}";
   Item : constant AdaPayload := Decode_AdaPayload (Text);
   Again : constant AdaPayload := Decode_AdaPayload (Encode_AdaPayload (Item));
begin
   if Item.Level /= -7 or else not Item.Enabled
     or else To_String (Item.Name) /= "smoke"
     or else Again.Level /= Item.Level or else Again.Name /= Item.Name
     or else Item.Note.Is_Present
     or else Service_API.Function_Codec.Exchange_Notice.Message_Name /= "AdaNotice"
   then
      raise Program_Error with "generated model/codec round trip failed";
   end if;
   begin
      declare
         Invalid : constant AdaPayload := Decode_AdaPayload ("{}");
      begin
         raise Program_Error with "accepted missing fields" & Invalid.Level'Image;
      end;
   exception
      when Codec_Error => null;
   end;
   Ada.Text_IO.Put_Line ("ADA SMOKE PASSED");
end Ada_Consumer;