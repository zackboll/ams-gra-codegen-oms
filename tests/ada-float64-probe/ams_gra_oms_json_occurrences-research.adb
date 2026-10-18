package body AMS_GRA_OMS_JSON_Occurrences.Research is
   function Numeric_Token (Item : Value) return String is
      use GNATCOLL.JSON;
   begin
      if Kind (Item) not in JSON_Int_Type | JSON_Float_Type then
         raise Validation_Error with "expected original numeric occurrence";
      end if;
      return Get (Get (Item.Node, "data"));
   end Numeric_Token;
end AMS_GRA_OMS_JSON_Occurrences.Research;