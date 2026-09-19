# The engine's classes under Godot, as C# names them: each is made at its
# first use from the engine's class database and inherits as it does there,
# up to Godot::Object, which the extension defines. Its `__`-prefixed
# methods are the extension's.
module Godot
  # What a call to the engine raises when the engine cannot run it.
  class CallError < StandardError; end

  class << self
    def const_missing(name)
      superclass = __engine_superclass__(name)
      return super if superclass.nil?

      engine_class = Class.new(const_get(superclass))
      engine_class.instance_variable_set(:@engine_class, true)
      const_set(name, engine_class)
    end

    private :__engine_superclass__

    private

    # What is defined under Godot is the engine's, never a file's, so a file
    # that raises takes none of it away.
    def const_added(name); end
  end

  # An engine object answers the engine's methods by their names.
  class Object
    @engine_class = true

    # What a class extending an engine class calls in its body. The file's
    # source is what Godot reads them from, so `tool` and `icon` do nothing
    # as the body runs; `abstract` marks only the class calling it, as
    # Active Record's `abstract_class` does.
    class << self
      def tool; end

      def icon(_path); end

      def abstract
        @abstract = true
      end

      # An engine class makes the engine's object; a node script's class
      # makes a node carrying its script, freed if it fails to initialize.
      def new(*args, **kwargs, &block)
        raise NotImplementedError, "#{self} is an abstract class and cannot be instantiated." if @abstract == true
        return __make__ if @engine_class == true

        node = __make_node__
        begin
          node.__send__(:initialize, *args, **kwargs, &block)
        rescue Exception # rubocop:disable Lint/RescueException
          node.free
          raise
        end
        node
      end

      private :__make__, :__make_node__, :__allocate__
    end

    def method_missing(name, *args)
      return super unless __engine_method__(name)

      __call__(name, args)
    end

    def respond_to_missing?(name, include_private = false)
      __engine_method__(name) || super
    end
  end
end
