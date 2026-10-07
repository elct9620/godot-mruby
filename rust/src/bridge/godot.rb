# The engine's classes under Godot, as C# names them: each is made at its
# first use from the engine's class database and inherits as it does there,
# up to Godot::Object, which the extension defines, and each of the engine's
# value types is a class under Godot::Value. Their `__`-prefixed methods are
# the extension's.
module Godot
  # What a call to the engine raises when the engine cannot run it.
  class CallError < StandardError; end

  # The engine's utility functions Ruby lacks, such as lerp and randf, each a
  # strict block of the number of arguments it takes, so a call gathers none
  # into an array; one taking any number gathers them.
  __utilities__.each do |name, arity|
    body = case arity
           when 0 then proc { __utility__(name) }
           when 1 then proc { |a| __utility__(name, a) }
           when 2 then proc { |a, b| __utility__(name, a, b) }
           when 3 then proc { |a, b, c| __utility__(name, a, b, c) }
           when 5 then proc { |a, b, c, d, e| __utility__(name, a, b, c, d, e) }
           else proc { |*args| __apply_utility__(name, args) }
           end
    singleton_class.__send__(:define_method, name, &body)
  end

  class << self
    def const_missing(name)
      superclass = __engine_superclass__(name)
      return const_set(name, Class.new(Value)) if superclass.nil? && Value.__send__(:__has_value_type__, name)
      return super if superclass.nil?

      engine_class = Class.new(const_get(superclass))
      engine_class.instance_variable_set(:@engine_class, true)
      const_set(name, engine_class)
    end

    private :__engine_superclass__, :__utilities__, :__utility__, :__apply_utility__

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
        return super unless __has_static_method__(name)

        __call_static__(name, args)
      end

      def respond_to_missing?(name, include_private = false)
        return super unless @engine_class == true

        singleton = engine_singleton
        singleton ? singleton.respond_to?(name) : __has_static_method__(name) || super
      end

      # The engine class's integer constants and enum values, kept on the
      # engine class once named.
      def const_missing(name)
        engine_class = __engine_class__
        value = engine_class.__send__(:__engine_constant__, name)
        return super if value.nil?

        engine_class.const_set(name, value)
      end

      private :__make__, :__make_node__, :__allocate__, :__singleton__, :__has_static_method__, :__call_static__,
              :__engine_constant__, :__declare_signal__, :__declare_export__, :__declare_heading__, :__shape__

      private

      # The nearest engine class among the ancestors: the class itself for an
      # engine class, the one it extends for a node class.
      def __engine_class__
        ancestors.find { |ancestor| ancestor.instance_variable_get(:@engine_class) == true }
      end

      # The body of the engine method `bound` binds: a strict block of the
      # arguments it requires and takes, so a call gathers none into an array,
      # an optional one left out standing as OMITTED for the engine's
      # default; a method taking any number, or more than four, gathers them.
      def __bound_body__(bound)
        case __shape__(bound)
        when [0, 0] then proc { __call_bound__(bound) }
        when [1, 0] then proc { |a| __call_bound__(bound, a) }
        when [2, 0] then proc { |a, b| __call_bound__(bound, a, b) }
        when [3, 0] then proc { |a, b, c| __call_bound__(bound, a, b, c) }
        when [4, 0] then proc { |a, b, c, d| __call_bound__(bound, a, b, c, d) }
        when [0, 1] then proc { |a = OMITTED| __call_bound__(bound, a) }
        when [1, 1] then proc { |a, b = OMITTED| __call_bound__(bound, a, b) }
        when [2, 1] then proc { |a, b, c = OMITTED| __call_bound__(bound, a, b, c) }
        when [3, 1] then proc { |a, b, c, d = OMITTED| __call_bound__(bound, a, b, c, d) }
        when [0, 2] then proc { |a = OMITTED, b = OMITTED| __call_bound__(bound, a, b) }
        when [1, 2] then proc { |a, b = OMITTED, c = OMITTED| __call_bound__(bound, a, b, c) }
        when [2, 2] then proc { |a, b, c = OMITTED, d = OMITTED| __call_bound__(bound, a, b, c, d) }
        when [0, 3] then proc { |a = OMITTED, b = OMITTED, c = OMITTED| __call_bound__(bound, a, b, c) }
        when [1, 3] then proc { |a, b = OMITTED, c = OMITTED, d = OMITTED| __call_bound__(bound, a, b, c, d) }
        when [0, 4] then proc { |a = OMITTED, b = OMITTED, c = OMITTED, d = OMITTED| __call_bound__(bound, a, b, c, d) }
        else proc { |*arguments| __apply_bound__(bound, arguments) }
        end
      end

      # The objects being inspected, so an instance variable leading back to
      # one of them is not inspected again.
      def __inspected__
        @__inspected__ ||= []
      end

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
        return ["type", __type_name__(keywords[:type])] if hint == :type
        return ["none", nil] if hint.nil?

        [hint.to_s, __hint_value__(hint, keywords[hint])]
      end

      # The value a hint is read with, a list where the hint takes the names
      # a value may take, which is refused where it is written otherwise.
      def __hint_value__(hint, value)
        if %i[enum flags].include?(hint) && !value.is_a?(::Array)
          raise ArgumentError, %("#{hint}:" needs a list of names, but #{value.inspect} was given instead.)
        end

        value
      end

      # The class a property names its type by, as the realm spells it. Only
      # a class names a type, so anything else is refused where it is
      # written.
      def __type_name__(named)
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

    # An engine method the engine class this Ruby class extends declares is
    # defined on that engine class at its first call, so later calls skip
    # method_missing, and call the method's bind when the engine gives one.
    # A node class's own method of the name stays its own, reaching the
    # engine's through super.
    def method_missing(name, *args, &block)
      target, declared, bound = __resolve__(name)
      return super if target.nil?

      engine_class = self.class.__send__(:__engine_class__)
      if bound
        engine_class.__send__(:define_method, name, &engine_class.__send__(:__bound_body__, bound))
        engine_class.instance_method(name).bind_call(self, *args)
      else
        engine_class.__send__(:define_method, name) { |*arguments| __call__(target, arguments) } if declared
        __call__(target, args)
      end
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

    # A copy of an engine object stands for the same engine object and
    # carries its instance variables; a node class's object is the only one
    # its node has, so it refuses.
    def dup
      __make_copy__("dup")
    end

    def clone
      copy = __make_copy__("clone")
      copy.freeze if frozen?
      copy
    end

    # The Ruby class, the engine object as Godot prints it, and the instance
    # variables as Ruby inspects them.
    def inspect
      inspected = Godot::Object.__send__(:__inspected__)
      head = "#<#{self.class} #{__label__}"
      return "#{head} ...>" if inspected.any? { |object| object.equal?(self) }

      inspected.push(self)
      variables = instance_variables.map { |name| "#{name}=#{instance_variable_get(name).inspect}" }
      variables.empty? ? "#{head}>" : "#{head} #{variables.join(", ")}>"
    ensure
      inspected.pop if inspected.last.equal?(self)
    end

    private

    def __make_copy__(verb)
      raise TypeError, "can't #{verb} #{self.class}" unless self.class.instance_variable_get(:@engine_class) == true

      copy = self.class.__send__(:__allocate__, self)
      instance_variables.each { |name| copy.instance_variable_set(name, instance_variable_get(name)) }
      copy
    end
  end

  # A value of one of the engine's value types, such as Vector2 or Color:
  # built by the engine's constructors, answering the engine's members,
  # methods and operators, and never changed, since each side holds its own
  # copy.
  class Value
    class << self
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

      private :__has_value_type__, :__call_static__, :__constant__

      private

      # What is defined on a value type's class is the engine's, never a file's.
      def const_added(name); end
    end

    # A Hash key matches only a value of its own type, as 4 never matches
    # 4.0, so keys of two types meet without the engine refusing to compare.
    def eql?(other)
      other.instance_of?(self.class) && self == other
    end

    def hash
      __hash__
    end

    # A copy is another value of the same class, holding the engine's copy
    # of this one and its instance variables.
    def dup
      copy = __copy_value__
      instance_variables.each { |name| copy.instance_variable_set(name, instance_variable_get(name)) }
      copy
    end

    def clone
      copy = dup
      copy.freeze if frozen?
      copy
    end

    def inspect
      "#<#{self.class} #{self}>"
    end

    # A member or engine method of the value's type is defined on its class
    # at the first call, so later calls skip method_missing.
    def method_missing(name, *args, &block)
      raise FrozenError, "can't modify #{self.class}: build a new one instead" if name.to_s[-1] == "="

      kind, bound = __resolve__(name)
      case kind
      when :member
        if bound
          self.class.__send__(:define_method, name) { __get__(bound) }
        else
          self.class.__send__(:define_method, name) { __member__(name) }
        end
      when :method then self.class.__send__(:define_method, name) { |*arguments| __call__(name, arguments) }
      else return super
      end
      __send__(name, *args, &block)
    end

    def respond_to_missing?(name, include_private = false)
      !__resolve__(name).nil? || super
    end
  end
end
