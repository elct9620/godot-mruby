# Exports a property naming a node script's class, and prints the class the
# game reads the property's hint as.
module Integration
  module Export
    class Lookout < Godot::Node
      export :quarry, nil, type: Seeker

      def _ready
        quarry = get_property_list.find { |property| property["name"] == "quarry" }
        puts "quarry hint: #{quarry && quarry['hint_string']}"
      end
    end
  end
end
