# The engine's classes under Godot, as C# names them: each is made at its
# first use from the engine's class database and inherits as it does there.
# `__engine_superclass__` is the extension's.
module Godot
  class << self
    def const_missing(name)
      superclass = __engine_superclass__(name)
      return super if superclass.nil?

      engine_class = superclass.empty? ? Class.new(::Object) { extend ClassBody } : Class.new(const_get(superclass))
      const_set(name, engine_class)
    end

    private :__engine_superclass__

    # What a class extending an engine class calls in its body. The file's
    # source is what Godot reads them from, so `tool` and `icon` do nothing
    # as the body runs; `abstract` marks only the class calling it, as
    # Active Record's `abstract_class` does.
    ClassBody = Module.new do
      def tool; end

      def icon(_path); end

      def abstract
        @abstract = true
      end

      def new(*args, **kwargs, &block)
        raise NotImplementedError, "#{self} is an abstract class and cannot be instantiated." if @abstract == true

        super
      end
    end

    private

    # What is defined under Godot is the engine's, never a file's, so a file
    # that raises takes none of it away.
    def const_added(name); end
  end
end
