with Ada.Text_IO;
with Programs.OAM; use Programs.OAM;
with Service_Codec; use Service_Codec;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with AMS_GRA_OMS_JSON_Guard;
procedure Combined_Consumer is
   Good : constant String :=
     "{""Pick"":{""Nested"":{""Numbers"":[3,-2,1],""Labels"":[""first"",""last""]}},"
     & """Values"":[9,8],""Rows"":[{""Numbers"":[4,5],""Labels"":[""row""]}]}";
   procedure Check (OK : Boolean) is
   begin
      if not OK then raise Program_Error with "combined codec regression"; end if;
   end Check;
   procedure Reject (Bad : String) is
   begin
      declare
         Item : constant CombinedPayload := Decode_CombinedPayload (Bad);
      begin
         raise Program_Error with "accepted malformed " & Item.Pick.Kind'Image;
      end;
   exception
      when Codec_Error => null;
   end Reject;
begin
   declare
      Item : constant CombinedPayload := Decode_CombinedPayload (Good);
      Again : constant CombinedPayload := Decode_CombinedPayload (Encode_CombinedPayload (Item));
   begin
      Check (Item.Pick.Kind = Nested_Kind and then Again.Pick.Kind = Nested_Kind);
      Check (Length (Again.Pick.Nested.Numbers) = 3);
      Check (Element (Again.Pick.Nested.Numbers, 1) = 3);
      Check (Element (Again.Pick.Nested.Numbers, 2) = -2);
      Check (Element (Again.Pick.Nested.Numbers, 3) = 1);
      Check (Value (Again.Pick.Nested.Labels.Required (1)) = "first");
      Check (Value (Element (Again.Pick.Nested.Labels.Additional, 1)) = "last");
      Check (AMS_GRA_OMS_JSON_Guard.Read (Good) =
             AMS_GRA_OMS_JSON_Guard.Read (Encode_CombinedPayload (Again)));
   end;
   Reject ("{""Pick"":{""Nested"":{""Numbers"":[1],""Labels"":[""ok""]}}}");
   Reject ("{""Pick"":{""Nested"":{""Numbers"":[1,2,3,4],""Labels"":[""ok""]}}}");
   Reject ("{""Pick"":{""Nested"":{""Numbers"":[1,2,128],""Labels"":[""ok""]}}}");
   Reject ("{""Pick"":{""Nested"":{""Numbers"":[1,2],""Labels"":[""ok"",""""]}}}");
   Reject ("{}");
   Reject ("{""Pick"":{}}");
   Reject ("{""Pick"":{""Unknown"":true}}");
   Reject ("{""Pick"":{""Enabled"":true,""Nested"":{}}}");
   Ada.Text_IO.Put_Line ("TASK089R_COMBINED_PASS");
end Combined_Consumer;