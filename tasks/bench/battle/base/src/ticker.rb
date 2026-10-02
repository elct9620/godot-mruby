# A node whose physics callback does nothing, so the time of a call is the
# time of entering Ruby.
class Ticker < Godot::Node
  def _physics_process(_delta); end
end
