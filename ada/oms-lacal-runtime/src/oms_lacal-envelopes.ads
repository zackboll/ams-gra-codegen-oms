package OMS_LACAL.Envelopes is
   function Wrap (Name, Payload : String) return String;
   --  Counts original members, including duplicates. Returns original payload
   --  bytes without numeric conversion or DOM reconstruction.
   function Extract (Name, Document : String) return String;
end OMS_LACAL.Envelopes;