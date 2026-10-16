module Unit
  module Declarations
    # Keeps slots no export declares, answering for them through the
    # virtuals GDScript's dynamic properties use, hides a property of its
    # node, and prints as its contents.
    class Chest < Godot::Node
      def initialize
        @slots = {}
      end

      def _get(property)
        name = property.to_s
        @slots[name] if name[0, 5] == "slot_"
      end

      def _set(property, value)
        name = property.to_s
        return false unless name[0, 5] == "slot_"

        @slots[name] = value
        true
      end

      def _get_property_list
        [{ "name" => "slot_1", "type" => 2, "usage" => 6 }]
      end

      def _validate_property(property)
        property["usage"] = 0 if property["name"].to_s == "editor_description"
      end

      def to_s
        "chest of #{@slots.size}"
      end
    end
  end
end
