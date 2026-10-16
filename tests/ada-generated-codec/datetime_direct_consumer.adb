with Ada.Text_IO; use Ada.Text_IO;
with Ada.Command_Line;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Guard;
with Urn.Test; use Urn.Test;
with Service_API;
with Service_Codec; use Service_Codec;

procedure Datetime_Direct_Consumer is
   Passed : Natural := 0;
   File : File_Type;
   Golden : Unbounded_String;
   Item : constant DirectPayload :=
     (Observed => Create ("2026-10-13T12:34:56.12300+05:30"),
      Timestamp => (True, Create ("2026-10-13T12:34:56")));
   procedure Check (Condition : Boolean) is
   begin
      if not Condition then raise Program_Error with "DateTime check failed"; end if;
      Passed := Passed + 1;
   end Check;
   procedure Reject (Text : String) is
   begin
      declare
         Result : constant DirectPayload := Decode_DirectPayload (Text);
      begin
         raise Program_Error with "accepted invalid DateTime " & Value (Result.Observed);
      end;
   exception
      when Codec_Error => Check (True);
   end Reject;
   procedure Wrong (Key : String; Replacement : JSON_Value) is
      Object : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read (To_String (Golden));
   begin
      Object.Set_Field (Key, Replacement);
      Reject (AMS_GRA_OMS_JSON_Guard.Write (Object));
   end Wrong;
begin
   Open (File, In_File, Ada.Command_Line.Argument (1));
   while not End_Of_File (File) loop Append (Golden, Get_Line (File)); end loop;
   Close (File);
   Check (AMS_GRA_OMS_JSON_Guard.Read (Encode_DirectPayload (Item)) =
          AMS_GRA_OMS_JSON_Guard.Read (To_String (Golden)));
   declare
      Result : constant DirectPayload := Decode_DirectPayload (To_String (Golden));
      Roundtrip : constant DirectPayload := Decode_DirectPayload (Encode_DirectPayload (Item));
      Absent : constant DirectPayload := (Item.Observed, (Is_Present => False));
      Empty : constant DirectPayload := Decode_DirectPayload (Encode_DirectPayload (Absent));
      Object : constant JSON_Value := AMS_GRA_OMS_JSON_Guard.Read (To_String (Golden));
   begin
      Check (Value (Result.Observed) = Value (Item.Observed));
      Check (Value (Roundtrip.Timestamp.Value) = Value (Item.Timestamp.Value));
      Check (not Empty.Timestamp.Is_Present);
      Check (not AMS_GRA_OMS_JSON_Guard.Read (Encode_DirectPayload (Absent)).Has_Field ("{urn:test}Timestamp"));
      Object.Unset_Field ("{urn:test}Observed");
      Reject (AMS_GRA_OMS_JSON_Guard.Write (Object));
   end;
   Reject ("[]"); Reject ("{}"); Reject ("null");
   Wrong ("unknown", GNATCOLL.JSON.Create (True));
   Wrong ("{urn:test}Observed", JSON_Null);
   Wrong ("{urn:test}Timestamp", JSON_Null);
   Wrong ("{urn:test}Observed", GNATCOLL.JSON.Create (False));
   Wrong ("{urn:test}Observed", GNATCOLL.JSON.Create (Long_Long_Integer'(42)));
   Wrong ("{urn:test}Timestamp", GNATCOLL.JSON.Create ("2026-10-13T12:34:56+14:01"));
   Reject ("{""{urn:test}Observed"":""\uD800""}");
   Reject ("{""{urn:test}Observed"":""" & Character'Val (255) & """}");
   Reject ((1 .. AMS_GRA_OMS_JSON_Guard.Max_Document_Bytes + 1 => ' '));
   Reject ((1 .. 129 => '[') & "0" & (1 .. 129 => ']'));
   Open (File, In_File, Ada.Command_Line.Argument (2));
   while not End_Of_File (File) loop
      declare
         Status : constant String := Get_Line (File);
         Text : constant String := Get_Line (File);
         Expected : constant String := Get_Line (File);
      begin
         if Status = "INVALID" then Reject (Text);
         else
            declare
               Result : constant DirectPayload := Decode_DirectPayload (Text);
               Again : constant DirectPayload := Decode_DirectPayload (Encode_DirectPayload (Result));
            begin
               Check (Value (Result.Observed) = Expected);
               Check (Value (Again.Observed) = Expected);
            end;
         end if;
      end;
   end loop;
   Close (File);
   Put_Line ("TASK090_DIRECT_COMPLETE cases=" & Passed'Image);
end Datetime_Direct_Consumer;