module Loader
  module Posts
    # Ruby finds this Sentry first, though the loader's rule finds sentry.rb.
    Sentry = Class.new(Godot::Node3D)

    class Shadowed < Sentry
    end
  end
end
