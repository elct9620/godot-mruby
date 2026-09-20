# The engine's classes under Godot, as C# names them: each is made at its
# first use from the engine's class database and inherits as it does there,
# up to Godot::Object, which the extension defines, and each of the engine's
# value types is a class under Godot::Value. Their `__`-prefixed methods are
# the extension's.
module Godot
  # What a call to the engine raises when the engine cannot run it.
  class CallError < StandardError; end

  # The engine's utility functions Ruby lacks, such as lerp and randf.
  __utilities__.each do |name|
    singleton_class.__send__(:define_method, name) { |*args| __utility__(name, args) }
  end

  class << self
    def const_missing(name)
      superclass = __engine_superclass__(name)
      return const_set(name, Class.new(Value)) if superclass.nil? && Value.__send__(:__value_type__, name)
      return super if superclass.nil?

      engine_class = Class.new(const_get(superclass))
      engine_class.instance_variable_set(:@engine_class, true)
      const_set(name, engine_class)
    end

    private :__engine_superclass__, :__utilities__, :__utility__

    private

    # What is defined under Godot is the engine's, never a file's, so a file
    # that raises takes none of it away.
    def const_added(name); end
  end

  # An engine object answers the engine's methods by their names.
  class Object
    @engine_class = true

    # What a class extending an engine class calls in its body. Godot reads
    # `tool` and `icon` from the file's source, so they do nothing as the
    # body runs; `abstract` marks only the class calling it, as Active
    # Record's `abstract_class` does; `signal` and `export` are taken by the
    # realm, which publishes them for Godot once the file has run.
    class << self
      def tool; end

      def icon(_path); end

      def abstract
        @abstract = true
      end

      # A signal of the class, named as Godot names it, with a name for each
      # value it is emitted with. A node of the class has it as the engine's
      # own signals, so it is connected to and emitted by their names.
      def signal(name, *parameters)
        __declare_signal__(name.to_s, parameters.map { |parameter| parameter.to_s })
      end

      # A property of the class, taking its type from the value it is
      # declared with. The engine and the editor read and write it by that
      # name, and the class keeps it in the instance variable of that name.
      # A name every Ruby object answers is refused, since the accessors
      # would take that answer away.
      def export(name, default)
        if ::Object.method_defined?(name)
          raise ArgumentError, "#{name} is a method every object answers, so it cannot be exported"
        end

        __declare_export__(name.to_s, default)
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

      # An engine class answers its singleton's methods, or its static ones.
      def method_missing(name, *args, &block)
        return super unless @engine_class == true

        singleton = engine_singleton
        return singleton.__send__(name, *args, &block) if singleton
        return super unless __static_method__(name)

        __call_static__(name, args)
      end

      def respond_to_missing?(name, include_private = false)
        return super unless @engine_class == true

        singleton = engine_singleton
        singleton ? singleton.respond_to?(name) : __static_method__(name) || super
      end

      # The engine class's integer constants and enum values, kept on the
      # engine class once named.
      def const_missing(name)
        engine_class = ancestors.find { |ancestor| ancestor.instance_variable_get(:@engine_class) == true }
        value = engine_class.__send__(:__engine_constant__, name)
        return super if value.nil?

        engine_class.const_set(name, value)
      end

      private :__make__, :__make_node__, :__allocate__, :__singleton__, :__static_method__, :__call_static__,
              :__engine_constant__, :__declare_signal__, :__declare_export__

      private

      # The engine's singleton of this engine class, kept once asked for.
      def engine_singleton
        @singleton = __singleton__ unless instance_variable_defined?(:@singleton)
        @singleton
      end

      # What is defined on an engine class is the engine's, never a file's.
      def const_added(name)
        super unless @engine_class == true
      end
    end

    # An engine method the engine class declares is defined on the Ruby class
    # at its first call, so later calls skip method_missing.
    def method_missing(name, *args, &block)
      target, declared = __resolve__(name)
      return super if target.nil?

      self.class.__send__(:define_method, name) { |*arguments| __call__(target, arguments) } if declared
      __call__(target, args)
    end

    def respond_to_missing?(name, include_private = false)
      !__resolve__(name).nil? || super
    end

    # Two Ruby objects for one engine object are equal.
    def ==(other)
      other.is_a?(Godot::Object) && __instance_id__ == other.__send__(:__instance_id__)
    end
    alias eql? ==

    def hash
      __instance_id__.hash
    end
  end

  # A value of one of the engine's value types, such as Vector2 or Color:
  # built by the engine's constructors, answering the engine's members,
  # methods and operators, and never changed, since each side holds its own
  # copy.
  class Value
    class << self
      def new(*args)
        __construct__(args)
      end

      def method_missing(name, *args, &block)
        answered = __call_static__(name, args)
        return super if answered.nil?

        answered.first
      end

      def const_missing(name)
        value = __constant__(name)
        return super if value.nil?

        const_set(name, value)
      end

      private :__value_type__, :__construct__, :__call_static__, :__constant__

      private

      # What is defined on a value type's class is the engine's, never a file's.
      def const_added(name); end
    end

    %i[+ - * / % ** < <= > >= -@ +@].each do |operator|
      define_method(operator) { |*other| __operate__(operator, other.first) }
    end

    def ==(other)
      other.is_a?(Value) && __operate__(:==, other)
    end
    alias eql? ==

    def hash
      __hash__
    end

    def inspect
      "#<#{self.class} #{self}>"
    end

    def method_missing(name, *args, &block)
      raise FrozenError, "can't modify #{self.class}: build a new one instead" if name.to_s[-1] == "="

      found = __member__(name) if args.empty?
      return found.first if found

      answered = __call__(name, args)
      return super if answered.nil?

      answered.first
    end

    def respond_to_missing?(name, include_private = false)
      !__member__(name).nil? || super
    end
  end
end
