module Unit
  module Loader
    module Posts
      # Ruby finds this Marker first, though the loader's rule finds marker.rb.
      Marker = Class.new(StandardError)

      class Plain < Marker
      end
    end
  end
end
