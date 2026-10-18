-- Research-only child: exposes the original node-owned lexeme, never a DOM
-- numeric conversion. No production occurrence interface is changed.
package AMS_GRA_OMS_JSON_Occurrences.Research is
   function Numeric_Token (Item : Value) return String;
end AMS_GRA_OMS_JSON_Occurrences.Research;