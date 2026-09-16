# The engine's classes under Godot, as C# names them: each is made at its
# first use from the engine's class database and inherits as it does there.
# `__engine_superclass__` is the extension's.
module Godot
  class << self
    def const_missing(name)
      superclass = __engine_superclass__(name)
      return super if superclass.nil?

      const_set(name, Class.new(superclass.empty? ? ::Object : const_get(superclass)))
    end

    private :__engine_superclass__

    private

    # What is defined under Godot is the engine's, never a file's, so a file
    # that raises takes none of it away.
    def const_added(name); end
  end
end
