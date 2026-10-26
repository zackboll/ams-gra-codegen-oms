with Ada.Strings.Fixed;
with Ada.Strings.Unbounded; use Ada.Strings.Unbounded;
with AMS_GRA_OMS_JSON_Guard;
with GNATCOLL.JSON; use GNATCOLL.JSON;
with OMS_LACAL.Envelopes;

package body OMS_LACAL.Connections is
   package Guard renames AMS_GRA_OMS_JSON_Guard;
   use type Transports.Receive_Status;

   procedure Check_Id (Value : String) is
   begin
      if not Valid_Identifier (Value) then
         raise Runtime_Error with "invalid OWP identifier";
      end if;
   end Check_Id;

   procedure Check_Call (Self : Connection; Need_Active : Boolean := True) is
   begin
      if Self.Dispatching or Self.Polling then
         raise Runtime_Error with "reentrant runtime operation";
      elsif Need_Active and then Self.Current /= Active then
         raise Runtime_Error with "connection not initialized";
      end if;
   end Check_Call;

   function Image (SID : Subscription_ID) return String is
     (Ada.Strings.Fixed.Trim (SID'Image, Ada.Strings.Both));

   function State (Self : Connection) return Connection_State is (Self.Current);
   function Busy (Self : Connection) return Boolean is (Self.Waiting /= None);
   function Lost_Events (Self : Connection) return Natural is (Self.Dropped);

   procedure Report
     (Self : in out Connection; Kind : Event_Kind; SID : Subscription_ID; Detail : String)
   is
      Item : constant Event := (Kind, SID, To_Unbounded_String
        (Detail (Detail'First .. Detail'First + Natural'Min (Detail'Length, 4096) - 1)));
   begin
      if Self.Count = Max_Events then
         Self.Queue (Self.Head) := (others => <>);
         Self.Head := Self.Head mod Max_Events + 1;
         Self.Count := Self.Count - 1;
         if Self.Dropped < Natural'Last then
            Self.Dropped := Self.Dropped + 1;
         end if;
      end if;
      Self.Queue ((Self.Head - 1 + Self.Count) mod Max_Events + 1) := Item;
      Self.Count := Self.Count + 1;
   end Report;

   procedure Next_Event (Self : in out Connection; Available : out Boolean; Item : out Event) is
   begin
      Check_Call (Self, False);
      Available := Self.Count > 0;
      Item := (others => <>);
      if Available then
         Item := Self.Queue (Self.Head);
         Self.Queue (Self.Head) := (others => <>);
         Self.Head := Self.Head mod Max_Events + 1;
         Self.Count := Self.Count - 1;
      end if;
   end Next_Event;

   procedure Forget (Self : in out Connection; SID : Subscription_ID) is
   begin
      Check_Call (Self, False);
      for E of Self.Table loop
         if E.SID = SID then
            E := (others => <>);
         end if;
      end loop;
   end Forget;

   procedure Stop (Self : in out Connection) is
   begin
      if Self.Current /= Closed then
         Self.Current := Closed;
         Self.Waiting := None;
         Self.Pending_SID := No_Subscription;
         Self.Table := (others => (others => <>));
         --  State and mappings are cleared even if the transport close fails.
         Report (Self, Connection_Closed, No_Subscription, "connection closed");
         Transports.Close (Self.Wire.all);
      end if;
   end Stop;

   procedure Close (Self : in out Connection) is
   begin
      Check_Call (Self, False);
      Stop (Self);
   end Close;

   overriding procedure Finalize (Self : in out Connection) is
   begin
      Stop (Self);
   exception
      when others => null; -- Finalization cannot report transport cleanup failure.
   end Finalize;

   procedure Send (Self : in out Connection; Frame : String; Kind : Pending_Kind;
                   SID : Subscription_ID := No_Subscription) is
   begin
      if Frame'Length > Max_Frame_Bytes then
         raise Runtime_Error with "OWP frame size limit";
      elsif Self.Waiting /= None then
         raise Runtime_Error with "acknowledgment outstanding";
      end if;
      begin
         Self.Polling := True;
         Transports.Send (Self.Wire.all, Frame);
         Self.Polling := False;
      exception
         when others =>
            Self.Polling := False;
            Stop (Self);
            raise;
      end;
      if Self.Verbose then
         Self.Waiting := Kind;
         Self.Pending_SID := SID;
      end if;
   end Send;

   procedure Initialize
     (Self : in out Connection; Schema, Service_ID : String; Verbose : Boolean := True)
   is
      Value : JSON_Value := Create_Object;
      Versions : JSON_Array;
   begin
      Check_Call (Self, False);
      if Self.Current /= New_Connection then
         raise Runtime_Error with "INIT must be first and unique";
      end if;
      Check_Id (Service_ID);
      if Schema'Length not in 1 .. 256 then
         raise Runtime_Error with "invalid schema version";
      end if;
      Append (Versions, Create ("1.0"));
      Set_Field (Value, "versions", Create (Versions));
      Set_Field (Value, "schema", Create (Schema));
      Set_Field (Value, "verbose", Create (Verbose));
      Set_Field (Value, "service_id", Create (Service_ID));
      Self.Verbose := Verbose;
      Send (Self, "INIT " & Guard.Write (Value), Init_Command);
      Self.Current := Initializing;
   end Initialize;

   procedure Publish
     (Self : in out Connection; Namespace, Name, Topic, Payload : String)
   is
   begin
      Check_Call (Self);
      Check_Id (Topic);
      declare
         Document : constant String := Envelopes.Wrap (QName (Namespace, Name), Payload);
      begin
         Send (Self, "PUB " & Topic & " " & Document, Publish_Command);
      end;
   end Publish;

   function Subscribe
     (Self : in out Connection; Namespace, Name, Topic : String;
      Has_Group : Boolean; Group : String) return Subscription_ID
   is
      Name_Text : constant String := QName (Namespace, Name);
      Slot : Natural := 0;
      SID : Subscription_ID;
   begin
      Check_Call (Self);
      Check_Id (Topic);
      if Has_Group then
         Check_Id (Group);
      elsif Group /= "" then
         raise Runtime_Error with "group supplied without presence flag";
      end if;
      if Self.Exhausted then
         raise Runtime_Error with "subscription IDs exhausted";
      end if;
      for I in Self.Table'Range loop
         if Self.Table (I).Status = Unused then Slot := I; exit; end if;
      end loop;
      if Slot = 0 then raise Runtime_Error with "subscription capacity"; end if;
      SID := Self.Next_SID;
      --  Allocate string before publication. All ordinary validation occurs
      --  before Send; the table does not become dispatchable before +OK.
      declare
         Prepared : constant Subscription_Entry :=
           (Status => (if Self.Verbose then Pending else Registered),
            SID => SID, Name => To_Unbounded_String (Name_Text));
      begin
         Send (Self, "SUB " & Image (SID) & " " & Name_Text & " " & Topic &
                 (if Has_Group then " " & Group else ""), Subscribe_Command, SID);
         Self.Table (Slot) := Prepared;
      end;
      if SID = Subscription_ID'Last then Self.Exhausted := True;
      else Self.Next_SID := SID + 1; end if;
      return SID;
   end Subscribe;

   function Is_Active (Self : Connection; SID : Subscription_ID) return Boolean is
     (Self.Current = Active and then
       (for some E of Self.Table => E.Status = Registered and then E.SID = SID));

   procedure Unsubscribe (Self : in out Connection; SID : Subscription_ID) is
   begin
      Check_Call (Self);
      if not Is_Active (Self, SID) then
         raise Runtime_Error with "unknown/inactive subscription";
      end if;
      Send (Self, "UNSUB " & Image (SID), Unsubscribe_Command, SID);
      Forget (Self, SID);
   end Unsubscribe;

   procedure Dispatch
     (Self : in out Connection; SID : Subscription_ID;
      Process : not null access procedure) is
   begin
      Check_Call (Self);
      if not Is_Active (Self, SID) then
         raise Runtime_Error with "inactive dispatch";
      end if;
      Self.Dispatching := True;
      begin
         Process.all;
      exception
         when others => Self.Dispatching := False; raise;
      end;
      Self.Dispatching := False;
   end Dispatch;

   procedure Check_Info (Document : String) is
      Value : constant JSON_Value := Guard.Read (Document);
      procedure String_Field (Object : JSON_Value; Name : String) is
      begin
         if Kind (Get (Object, Name)) /= JSON_String_Type then
            raise Runtime_Error with "INFO missing string " & Name;
         end if;
      end String_Field;
      U : JSON_Value;
      type Names is array (Positive range <>) of String (1 .. 12);
      procedure UUID_Field (Object : JSON_Value; Name : String) is
      begin
         String_Field (Object, Name);
         declare
            S : constant String := Get (Get (Object, Name));
         begin
            if S'Length /= 36 then raise Runtime_Error with "invalid INFO UUID"; end if;
            for I in S'Range loop
               if I in 9 | 14 | 19 | 24 then
                  if S (I) /= '-' then raise Runtime_Error with "invalid INFO UUID"; end if;
               elsif S (I) not in '0' .. '9' | 'a' .. 'f' | 'A' .. 'F' then
                  raise Runtime_Error with "invalid INFO UUID";
               end if;
            end loop;
         end;
      end UUID_Field;
      procedure UUID_Member (Name : String; Child : JSON_Value) is
         pragma Unreferenced (Name);
         Wrapper : constant JSON_Value := Create_Object;
      begin
         Set_Field (Wrapper, "id", Child);
         UUID_Field (Wrapper, "id");
      end UUID_Member;
   begin
      if Kind (Value) /= JSON_Object_Type then raise Runtime_Error with "INFO not object"; end if;
      String_Field (Value, "version");
      if String'(Get (Get (Value, "version"))) /= "1.0" then
         raise Runtime_Error with "unsupported OWP version";
      end if;
      String_Field (Value, "server_id");
      Check_Id (Get (Get (Value, "server_id")));
      String_Field (Value, "system_label");
      U := Get (Value, "uuids");
      if Kind (U) /= JSON_Object_Type then raise Runtime_Error with "INFO uuids not object"; end if;
      UUID_Field (U, "system");
      UUID_Field (U, "service");
      if Has_Field (U, "subsystem") then UUID_Field (U, "subsystem"); end if;
      for Name of Names'(1 => "capabilities", 2 => "components  ") loop
         declare
            Key : constant String := Ada.Strings.Fixed.Trim (Name, Ada.Strings.Both);
         begin
            if Has_Field (U, Key) then
               if Kind (Get (U, Key)) /= JSON_Object_Type then
                  raise Runtime_Error with "INFO UUID mapping not object";
               end if;
               Map_JSON_Object (Get (U, Key), UUID_Member'Access);
            end if;
         end;
      end loop;
      if Has_Field (Value, "connect_urls") then
         if Kind (Get (Value, "connect_urls")) /= JSON_Array_Type then
            raise Runtime_Error with "INFO URLs not array";
         end if;
         declare URLs : constant JSON_Array := Get (Get (Value, "connect_urls"));
         begin
            for I in 1 .. Length (URLs) loop
               if Kind (Get (URLs, I)) /= JSON_String_Type then
                  raise Runtime_Error with "INFO URL not string";
               end if;
            end loop;
         end;
      end if;
   end Check_Info;

   procedure Poll (Self : in out Connection; Message : out Delivery) is
      Status : Transports.Receive_Status;
      Frame : Text;
      procedure Bad (Reason : String) is
      begin
         Report (Self, Protocol_Error, No_Subscription, Reason);
         Stop (Self);
      end Bad;
      procedure Process (Input : String) is
         F : constant String (1 .. Input'Length) := Input;
      begin
         if F'Length > Max_Frame_Bytes then
            Bad ("oversized OWP input");
         elsif F = "+OK" then
            if not Self.Verbose or Self.Waiting = None then
               Bad ("unexpected +OK");
               return;
            end if;
            if Self.Waiting = Subscribe_Command then
               for E of Self.Table loop
                  if E.Status = Pending and E.SID = Self.Pending_SID then
                     E.Status := Registered;
                  end if;
               end loop;
            end if;
            Report (Self, Command_Accepted, Self.Pending_SID, "+OK");
            Self.Waiting := None;
            Self.Pending_SID := No_Subscription;
         elsif F'Length > 5 and then F (1 .. 5) = "-ERR " then
            if Self.Current = New_Connection then
               Bad ("-ERR before INIT"); return;
            end if;
            declare
               Sep : Natural := Ada.Strings.Fixed.Index (F (6 .. F'Last), " ");
            begin
               if Sep = 0 then Sep := F'Last + 1; end if;
               declare Code : constant String := F (6 .. Sep - 1);
               begin
                  if Code /= "Unsupported-Version" and Code /= "Unsupported-Schema"
                    and Code /= "Unsupported-Service" and Code /= "Illegal-Operation"
                    and Code /= "Illegal-Argument" and Code /= "Illegal-State"
                    and Code /= "Internal-Error" and Code /= "Invalid-Message"
                  then Bad ("unknown OWP error code"); return; end if;
               end;
            end;
            Report (Self, Server_Error, Self.Pending_SID, F (6 .. F'Last));
            if Self.Waiting = Subscribe_Command then
               for E of Self.Table loop
                  if E.SID = Self.Pending_SID then E := (others => <>); end if;
               end loop;
            end if;
            Self.Waiting := None;
            Self.Pending_SID := No_Subscription;
            if Self.Current = Initializing then Stop (Self); end if;
         elsif F'Length > 5 and then F (1 .. 5) = "INFO " then
            if Self.Current not in Initializing | Active or Self.Waiting = Init_Command then
               Bad ("unexpected INFO or missing INIT +OK");
               return;
            end if;
            begin
               Check_Info (F (6 .. F'Last));
            exception
               when Guard.Validation_Error | Runtime_Error | Constraint_Error =>
                  Bad ("invalid INFO"); return;
            end;
            if Self.Current = Initializing then
               Self.Current := Active;
               Report (Self, Initialized, No_Subscription, F (6 .. F'Last));
            else
               Report (Self, Information, No_Subscription, F (6 .. F'Last));
            end if;
         elsif F'Length > 4 and then F (1 .. 4) = "MSG " then
            if Self.Current /= Active then Bad ("MSG before initialization"); return; end if;
            declare
               Sep : constant Natural := Ada.Strings.Fixed.Index (F (5 .. F'Last), " ");
               SID : Subscription_ID;
            begin
               if Sep = 0 or else Sep = F'Last then Bad ("malformed MSG"); return; end if;
               if not Valid_Identifier (F (5 .. Sep - 1)) then Bad ("invalid SID"); return; end if;
               --  Foreign legal SIDs are unknown, never converted with 'Value.
               for E of Self.Table loop
                  if E.Status = Registered and then Image (E.SID) = F (5 .. Sep - 1) then
                     SID := E.SID;
                     Message := (True, SID, E.Name, To_Unbounded_String (F (Sep + 1 .. F'Last)));
                     return;
                  end if;
               end loop;
               Report (Self, Unknown_Subscription, No_Subscription, F (5 .. Sep - 1));
            end;
         else
            Bad ("malformed/unknown OWP frame");
         end if;
      end Process;
      function Normalize (Input : String) return String is
         F : constant String (1 .. Input'Length) := Input;
         P : Positive := 1;
         First, Last : Natural;
         procedure Skip is
         begin
            while P <= F'Last and then F (P) in ' ' | ASCII.HT loop P := P + 1; end loop;
         end Skip;
         procedure Token is
         begin
            First := P;
            while P <= F'Last and then F (P) not in ' ' | ASCII.HT loop P := P + 1; end loop;
            Last := P - 1;
         end Token;
      begin
         Skip;
         Token;
         declare Op : constant String := F (First .. Last);
         begin
            Skip;
            if Op = "MSG" or Op = "-ERR" then
               Token;
               declare SID : constant String := F (First .. Last);
               begin
                  Skip;
                  return Op & " " & SID &
                    (if Op = "MSG" or P <= F'Last then " " & F (P .. F'Last) else "");
               end;
            end if;
            return Op & (if P <= F'Last then " " & F (P .. F'Last) else "");
         end;
      end Normalize;
   begin
      Check_Call (Self, False);
      Message := (others => <>);
      if Self.Current = Closed then return; end if;
      Self.Polling := True;
      begin
         Transports.Poll (Self.Wire.all, Status, Frame);
      exception
         when others => Self.Polling := False; Stop (Self); raise;
      end;
      if Status = Transports.Peer_Closed then Stop (Self);
      elsif Status = Transports.Frame_Ready then
         if Length (Frame) > Max_Frame_Bytes then Bad ("oversized OWP input");
         else Process (Normalize (To_String (Frame))); end if;
      end if;
      Self.Polling := False;
   exception
      when others => Self.Polling := False; raise;
   end Poll;
end OMS_LACAL.Connections;