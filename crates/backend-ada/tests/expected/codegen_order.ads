with Ada.Strings.Unbounded;

package Example.Order is

   type Optional_String (Is_Present : Boolean := False) is record
      case Is_Present is
         when False => null;
         when True  => Value : Ada.Strings.Unbounded.Unbounded_String;
      end case;
   end record;

   type Included_Id is range 1 .. 65_535;

   type Included_Quality is
     (Unknown,
      Good,
      Bad);

   type Record_First_In_Source is record
      Id : Included_Id;
      Quality : Included_Quality;
   end record;

end Example.Order;
