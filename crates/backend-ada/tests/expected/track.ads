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

   subtype Track_Sensor_Ids_Item is Long_Long_Integer;
   subtype Track_Sensor_Ids_Index is Positive range 1 .. 8;
   type Track_Sensor_Ids_Values is
     array (Positive range <>) of Track_Sensor_Ids_Item;

   type Track_Sensor_Ids_Sequence is private;

   --  The ONLY way to establish occupancy. Raises Constraint_Error
   --  unless 0 .. 8 values are supplied; nothing is truncated
   --  and no logical position is fabricated.
   function To_Sequence
     (Values : Track_Sensor_Ids_Values) return Track_Sensor_Ids_Sequence;
   function Length
     (Container : Track_Sensor_Ids_Sequence) return Natural;
   function Element
     (Container : Track_Sensor_Ids_Sequence; Index : Positive)
      return Track_Sensor_Ids_Item;
   procedure Append
     (Container : in out Track_Sensor_Ids_Sequence;
      New_Item  : Track_Sensor_Ids_Item);
   procedure Clear (Container : in out Track_Sensor_Ids_Sequence);

   type Track is record
      Id : Track_Id;
      Quality : Track_Quality;
      Callsign : Optional_String;
      Sensor_Ids : Track_Sensor_Ids_Sequence;
   end record;

private

   type Track_Sensor_Ids_Slot (Is_Used : Boolean := False) is record
      case Is_Used is
         when False => null;
         when True  => Value : Track_Sensor_Ids_Item;
      end case;
   end record;

   type Track_Sensor_Ids_Array is
     array (Track_Sensor_Ids_Index) of Track_Sensor_Ids_Slot;

   type Track_Sensor_Ids_Sequence is record
      --  Zero is a legal occupancy, so the default is the empty
      --  sequence over entirely unused slots. An unused slot has no
      --  payload component at all, so nothing is default-created and
      --  no placeholder value is invented.
      Count : Natural range 0 .. 8 := 0;
      Items : Track_Sensor_Ids_Array;
   end record;

end Oms.Track;
