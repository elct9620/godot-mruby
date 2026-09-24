module Unit
  module Loader
    module Rack
      # Makes Rack exist before the nut outside it loads.
      class Stand
      end
    end
  end
end
