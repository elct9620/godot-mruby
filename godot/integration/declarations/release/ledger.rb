# A class exporting a value whose node prints the categories and the exports
# it lists its properties with, then quits; a release build lists no
# category of the class.
module Integration
  module Declarations
    module Release
      class Ledger < Godot::Node
        # @GlobalScope's PropertyUsageFlags, which Ruby has no names for.
        CATEGORY = 128
        SCRIPT_VARIABLE = 4096

        export :balance, 10

        def _ready
          get_property_list.each do |property|
            usage = property["usage"]
            puts "category: #{property["name"]}" if usage & CATEGORY != 0
            puts "export: #{property["name"]}" if usage & SCRIPT_VARIABLE != 0
          end
          get_tree.quit
        end
      end
    end
  end
end
