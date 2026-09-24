with Ada.Strings.Unbounded;

package Oms.Track is

   type Optional_String (Is_Present : Boolean := False) is record
      case Is_Present is
         when False => null;
         when True  => Value : Standard.Ada.Strings.Unbounded.Unbounded_String;
      end case;
   end record;

   type Track_Id is range 1 .. 65_535;

   type Track_Quality is
     (Unknown,
      Tentative,
      Confirmed);

   type Track_Sensor_Ids_Slot (Is_Used : Boolean := False) is record
      case Is_Used is
         when False => null;
         when True  => Value : Long_Long_Integer;
      end case;
   end record;
   type Track_Sensor_Ids_Array is
     array (Positive range 1 .. 8) of Track_Sensor_Ids_Slot;
   type Track_Sensor_Ids_Sequence is record
      Length : Natural range 0 .. 8 := 0;
      Items  : Track_Sensor_Ids_Array;
   end record;

   type Track is record
      Id : Track_Id;
      Quality : Track_Quality;
      Callsign : Optional_String;
      Sensor_Ids : Track_Sensor_Ids_Sequence;
   end record;

end Oms.Track;
