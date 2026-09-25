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
      # would take that answer away. A keyword tells the editor how to show
      # the property, each one named as the `@export_*` annotation it
      # answers to: `range:` with a Range and an optional `step:`, `enum:`
      # and `flags:` with the names a value may take, `file:` with the
      # files to choose among, `dir:` and `multiline:` with `true`, and
      # `placeholder:` with the text an empty field shows. `type:` names the
      # class of an object Godot fills in, in place of a value naming the
      # type itself.
      def export(name, default, **keywords)
        if ::Object.method_defined?(name)
          raise ArgumentError, "#{name} is a method every object answers, so it cannot be exported"
        end

        __declare_export__(name.to_s, default, *__hint__(keywords))
        __exports__[name.to_sym] = default
        attr_reader name unless method_defined?(name)
        attr_writer name unless method_defined?(:"#{name}=")
      end

      # A heading the editor shows the properties written after it under,
      # named as GDScript's `@export_group` and its kin are. A group and a
      # subgroup take only the properties whose names begin with `prefix`,
      # and a category heads everything the class writes after it.
      def export_group(name, prefix = "")
        __declare_heading__(name.to_s, prefix.to_s, "group")
      end

      def export_subgroup(name, prefix = "")
        __declare_heading__(name.to_s, prefix.to_s, "subgroup")
      end

      def export_category(name)
        __declare_heading__(name.to_s, "", "category")
      end

      # An engine class makes the engine's object; a node script's class
      # makes a node carrying its script, freed if it fails to initialize.
      def new(*args, **kwargs, &block)
        raise NotImplementedError, "#{self} is an abstract class and cannot be instantiated." if @abstract == true
        return __make__ if @engine_class == true

        node = __write_defaults__(__make_node__)
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
              :__engine_constant__, :__declare_signal__, :__declare_export__, :__declare_heading__

      private

      # The class's object for a node the engine made, ready to initialize.
      def __build__(owner)
        __write_defaults__(__allocate__(owner))
      end

      # `object` with the value each exported property was declared with in
      # the instance variable of its name, written before anything
      # initializes it. A value that can change in place is copied, so no
      # two objects share one; the engine's own values never change, so they
      # are shared as they are.
      def __write_defaults__(object)
        __defaults__.each { |name, value| object.instance_variable_set(:"@#{name}", __copy__(value)) }
        object
      end

      # `value`, copied when it can change in place.
      def __copy__(value)
        value.is_a?(::Array) || value.is_a?(::Hash) || value.is_a?(::String) ? value.dup : value
      end

      # The value each property an object of the class is given, what it
      # inherits first, since a class exports no name its ancestors did.
      def __defaults__
        inherited = superclass.respond_to?(:__defaults__, true) ? superclass.__send__(:__defaults__) : {}
        inherited.merge(__exports__)
      end

      # What the class itself exported, by name, with the value each was
      # declared with.
      def __exports__
        @__exports__ ||= {}
      end

      # Takes back what the class exported, before its file runs again and
      # exports what its source says now.
      def __withdraw__
        @__exports__ = nil
      end

      # Gives `object`, made before the class's file ran again, the value of
      # each property the class now exports that it holds nothing for.
      def __adopt__(object)
        __exports__.each do |name, value|
          next if object.instance_variable_defined?(:"@#{name}")

          object.instance_variable_set(:"@#{name}", __copy__(value))
        end
      end

      # The hint the keywords name and the value it is read with, as the
      # extension takes them. A property carries one hint, as a GDScript
      # variable carries one `@export_*` annotation, and `step:` belongs to
      # the range it steps through rather than being a hint of its own.
      def __hint__(keywords)
        step = keywords[:step]
        named = keywords.keys.reject { |keyword| keyword == :step }
        raise ArgumentError, %("#{named.last}:" cannot be used with another hint.) if named.size > 1

        hint = named.first
        return ["range", __bounds__(keywords[:range], step)] if hint == :range
        raise ArgumentError, %("step:" needs a "range:" to step through.) unless step.nil?
        return ["type", __class_named__(keywords[:type])] if hint == :type
        return ["none", nil] if hint.nil?

        [hint.to_s, __listed__(hint, keywords[hint])]
      end

      # The value a hint is read with, a list where the hint takes the names
      # a value may take, which is refused where it is written otherwise.
      def __listed__(hint, value)
        if %i[enum flags].include?(hint) && !value.is_a?(::Array)
          raise ArgumentError, %("#{hint}:" needs a list of names, but #{value.inspect} was given instead.)
        end

        value
      end

      # The class a property names its type by, as the realm spells it. Only
      # a class names a type, so anything else is refused where it is
      # written.
      def __class_named__(named)
        unless named.is_a?(::Module)
          raise ArgumentError, %("type:" needs a class, but #{named.inspect} was given instead.)
        end

        named.to_s
      end

      # A range hint's bounds, the step included when one is declared. A
      # range leaving out its end names no highest value, so it is refused
      # where it is written.
      def __bounds__(range, step)
        unless range.is_a?(::Range) && !range.exclude_end?
          raise ArgumentError,
                %("range:" needs a range that includes its end, but #{range.inspect} was given instead.)
        end

        step.nil? ? [range.begin, range.end] : [range.begin, range.end, step]
      end

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

    # What Godot writes to a name the class did not export: the instance
    # variable of that name, if the object has one, as GDScript reaches a
    # member its script did not export. A name the object never wrote is
    # none of Ruby's, so the engine is left to answer it.
    def __write_variable__(name, value)
      variable = :"@#{name}"
      return false unless instance_variable_defined?(variable)

      instance_variable_set(variable, value)
      true
    end

    # What Godot reads from a name the class did not export, held in an
    # array so a variable holding nil is still an answer.
    def __read_variable__(name)
      variable = :"@#{name}"
      instance_variable_defined?(variable) ? [instance_variable_get(variable)] : nil
    end

    private :__write_variable__, :__read_variable__

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
