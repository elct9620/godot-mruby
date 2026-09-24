module Unit
  module Loader
    module Bay
      # Bay's own latch, whose file raises once it has defined the class.
      class Latch
      end

      raise "bay/latch.rb fails as a latch outside Bay loads"
    end
  end
end
