-- HANDWRITTEN, shape-specific research adapter over unmodified generated models.
with Ada.Strings.Unbounded;
with Programs.Oam;
package Task096_Adapter is
   package Model renames Programs.Oam;
   Codec_Error : exception;
   procedure Decode (Text : String; Output : in out Model.FloatPayload);
   procedure Encode (Item : Model.FloatPayload;
                     Output : in out Ada.Strings.Unbounded.Unbounded_String);
end Task096_Adapter;