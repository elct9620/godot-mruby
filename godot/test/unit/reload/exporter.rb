module Unit
  module Reload
    # A node script exporting a property its changed source leaves out.
    class Exporter < Godot::Node
      export :speed, 10

      def speed?
        instance_variable_defined?(:@speed)
      end
    end
  end
end
