module Loader
  # A node class Ruby makes with the brightness it shines at.
  class Beacon < Godot::Node2D
    attr_reader :brightness, :notified

    def initialize(brightness)
      @brightness = brightness
    end

    def _notification(what)
      @notified = what
    end
  end
end
