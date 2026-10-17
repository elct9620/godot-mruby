# The engine's classes under Godot, as C# names them: each is made at its
# first use from the engine's class database and inherits as it does there,
# up to Godot::Object, which the extension defines, and each of the engine's
# value types is a class under Godot::Value. Their `__`-prefixed methods are
# the extension's.
module Godot
  # What a call to the engine raises when the engine cannot run it.
  class CallError < StandardError; end

  # Calls `method` with `args` on the class the constant path `name` spells,
  # as GDScript calls `new` or a static function on a script.
  def self.__call_class__(name, method, *args)
    name.split("::").reduce(::Object) { |scope, part| scope.const_get(part) }.__send__(method, *args)
  end

  # The engine's utility functions Ruby lacks, such as lerp and randf, each a
  # strict block of the number of arguments it takes, naming its function by
  # its row, so a call neither gathers its arguments into an array nor looks
  # the function up by name; one taking any number gathers them.
  __utilities__.each do |name, arity, row|
    body = case arity
           when 0 then proc { __call_utility__(row) }
           when 1 then proc { |a| __call_utility__(row, a) }
           when 2 then proc { |a, b| __call_utility__(row, a, b) }
           when 3 then proc { |a, b, c| __call_utility__(row, a, b, c) }
           when 5 then proc { |a, b, c, d, e| __call_utility__(row, a, b, c, d, e) }
           else proc { |*args| __apply_utility__(row, args) }
           end
    singleton_class.__send__(:define_method, name, &body)
  end

  class << self
    def const_missing(name)
      superclass = __engine_superclass_by_name__(name)
      if superclass.nil? && Value.__send__(:__has_value_type__, name)
        value_class = const_set(name, Class.new(Value))
        value_class.__send__(:__define_engine_names__)
        return value_class
      end
      return super if superclass.nil?

      engine_class = Class.new(const_get(superclass))
      engine_class.instance_variable_set(:@engine_class, true)
      const_set(name, engine_class)
    end

    private :__engine_superclass_by_name__, :__utilities__, :__call_utility__, :__apply_utility__

    private

    # What is defined under Godot is the engine's, never a file's, so a file
    # that raises takes none of it away.
    def const_added(name); end
  end

  # An engine object answers the engine's methods by their names.
  class Object
    @engine_class = true

    # What a tool button's property holds for each object: a Callable of the
    # object's own method of that name.
    class ButtonMethod
      attr_reader :name

      def initialize(name)
        @name = name
      end
    end

    # What a typed container's property holds for each object: a copy of
    # the declared container, typed as the engine types it.
    class ContainerDefault
      attr_reader :value, :types

      def initialize(value, types)
        @value = value
        @types = types
      end
    end

    # What a class extending an engine class calls in its body. Godot reads
    # `tool` and `icon` from the file's source, so they do nothing as the
    # body runs; `abstract` marks only the class calling it, as Active
    # Record's `abstract_class` does, or the methods it is given; `signal`,
    # `rpc` and `export` are taken by the realm, which publishes them for
    # Godot once the file has run.
    class << self
      def tool; end

      def icon(_path); end

      # Marks the class as one never made, or each method named as one a
      # class extending it writes, as GDScript's `@abstract func` is: called,
      # it raises. Answers what it is given, so `abstract def` reads as one;
      # a method `def self.` defines on the class itself is left as it is.
      def abstract(*methods)
        return @abstract = true if methods.empty?

        methods.each do |method|
          next if !method_defined?(method) && singleton_class.method_defined?(method)

          define_method(method) do |*|
            raise NotImplementedError, %(Cannot call the parent class' abstract function "#{method}()" because it hasn't been defined.)
          end
        end
        methods.size == 1 ? methods.first : methods
      end

      # A signal of the class, named as Godot names it, with a keyword for
      # each value it is emitted with, naming the class of that value as
      # GDScript types a parameter: Godot::Bool for true and false, and
      # Godot::Variant for any value. A node of the class has it as the
      # engine's own signals, so it is connected to and emitted by their
      # names.
      def signal(name, **parameters)
        parameter_types = parameters.map { |parameter, type| [parameter.to_s, __check_class__(parameter, type)] }
        __declare_signal__(name.to_s, parameter_types)
      end

      # Lets peers call the method `name` over the network, as GDScript's
      # `@rpc` does, taking the words and channel it takes.
      def rpc(name, mode: :authority, sync: :call_remote, transfer_mode: :reliable, transfer_channel: 0)
        __declare_rpc__(name.to_s, [mode.to_s, sync.to_s, transfer_mode.to_s], transfer_channel)
      end

      # A property of the class, taking its type from the value it is
      # declared with. The engine and the editor read and write it by that
      # name, and the class keeps it in the instance variable of that name.
      # A name every Ruby object answers is refused, since the accessors
      # would take that answer away. A keyword tells the editor how to show
      # the property, each one named as the `@export_*` annotation it
      # answers to and taking what that annotation takes: `range:` with a
      # Range, an optional `step:` and `extra_hints:`; `enum:`, `flags:`,
      # `exp_easing:` and `node_path:` with a list; `file:`, `file_path:`
      # and `global_file:` with their filters; `placeholder:` and
      # `tool_button:` with their text, a tool button declared with the name
      # of the method it calls; `custom:` with a hint, what it is read with
      # and its usage; and the others with `true`. `type:` names the class of
      # an object Godot fills in, or in an array or a hash the classes a
      # typed container holds.
      def export(name, default, **keywords)
        if ::Object.method_defined?(name)
          raise ArgumentError, "#{name} is a method every object answers, so it cannot be exported"
        end

        hint, written = __read_hint__(keywords)
        if hint == "tool_button" && !default.is_a?(::Symbol)
          raise ArgumentError, %("tool_button:" needs the name of a method, but #{default.inspect} was given instead.)
        end
        types = __declare_export__(name.to_s, hint == "tool_button" ? nil : default, hint, written)
        __exports__[name.to_sym] = __read_default__(default, hint, types)
        attr_reader name unless method_defined?(name)
        __define_export_writer__(name, types) unless method_defined?(:"#{name}=")
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

      # An engine class makes the engine's object; a script's class makes an
      # object carrying its script, freed if it fails to initialize: a
      # reference-counted one once nothing holds it.
      def new(*args, **kwargs, &block)
        raise NotImplementedError, "#{self} is an abstract class and cannot be instantiated." if @abstract == true
        return __make__ if @engine_class == true

        object = __write_defaults__(__make_scripted__)
        begin
          object.__send__(:initialize, *args, **kwargs, &block)
        rescue Exception # rubocop:disable Lint/RescueException
          object.free unless object.is_a?(Godot::RefCounted)
          raise
        end
        object
      end

      # An engine class answers its singleton's methods, or its static ones,
      # each defined on the class at its first call, so later calls skip
      # method_missing, as an engine object's methods are.
      def method_missing(name, *args, &block)
        return super unless @engine_class == true

        singleton = engine_singleton
        if singleton
          __define_singleton_answer__(name) if singleton.respond_to?(name)
          return singleton.__send__(name, *args, &block)
        end
        unless __has_static_method__(name)
          __refuse_instance_method__(name)
          return super
        end

        __define_static_answer__(name)
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
        value = engine_class.__send__(:__engine_constant_by_name__, name)
        return super if value.nil?

        engine_class.const_set(name, value)
      end

      private :__make__, :__make_scripted__, :__allocate__, :__has_engine_member__, :__singleton__, :__has_static_method__, :__call_static__,
              :__refuse_instance_method__, :__engine_constant_by_name__, :__declare_signal__, :__declare_export__, :__declare_heading__, :__read_shape__

      private

      # The nearest engine class among the ancestors: the class itself for an
      # engine class, the one it extends for a node class.
      def __engine_class__
        ancestors.find { |ancestor| ancestor.instance_variable_get(:@engine_class) == true }
      end

      # The body of the engine method `bound` binds: a strict block of the
      # arguments it requires, so a call gathers none into an array, when it
      # takes no others and at most four; any other method gathers them.
      def __make_bound_body__(bound)
        case __read_shape__(bound)
        when [0, 0] then proc { __call_bound__(bound) }
        when [1, 0] then proc { |a| __call_bound__(bound, a) }
        when [2, 0] then proc { |a, b| __call_bound__(bound, a, b) }
        when [3, 0] then proc { |a, b, c| __call_bound__(bound, a, b, c) }
        when [4, 0] then proc { |a, b, c, d| __call_bound__(bound, a, b, c, d) }
        else proc { |*arguments| __apply_bound__(bound, arguments) }
        end
      end

      # Defines the singleton's method `name`, or the static method `name`, as
      # the class's own method. It answers only on this class: a class
      # extending it reaches method_missing as it did before, so what it
      # answers never turns on what was called first.
      def __define_singleton_answer__(name)
        owner = self
        define_singleton_method(name) do |*arguments, &body|
          next method_missing(name, *arguments, &body) unless equal?(owner)

          engine_singleton.__send__(name, *arguments, &body)
        end
      end

      def __define_static_answer__(name)
        owner = self
        define_singleton_method(name) do |*arguments, &body|
          next method_missing(name, *arguments, &body) unless equal?(owner)

          __call_static__(name, arguments)
        end
      end

      # The objects being inspected, so an instance variable leading back to
      # one of them is not inspected again.
      def __inspection_stack__
        @__inspection_stack__ ||= []
      end

      # The class's object for a node the engine made, ready to initialize.
      def __build__(owner)
        __write_defaults__(__allocate__(owner))
      end

      # `object` with the value each exported property was declared with in
      # the instance variable of its name, written before anything
      # initializes it. A value that can change in place is copied, so no
      # two objects share one; the engine's values of fixed size never
      # change, so they are shared as they are.
      def __write_defaults__(object)
        __defaults__.each { |name, value| object.instance_variable_set(:"@#{name}", __copy__(value, object)) }
        object
      end

      # `value`, copied for `object` when it can change in place: Ruby's
      # Array or Hash into the engine's container, as an export holds it, a
      # typed container typed again, and a tool button's method as the
      # object's own.
      def __copy__(value, object)
        case value
        when ButtonMethod then object.method(value.name)
        when ContainerDefault then __type__(__copy__(value.value, object), value.types)
        when ::Array, ::Hash then __hold__(value)
        when ::String, Godot::Dictionary, Godot::Value::Sequence then value.dup
        else value
        end
      end

      # `value` as an exported property holds it: Ruby's Array or Hash as
      # the engine's container of its elements, so GDScript reaching the
      # property changes what Ruby reads, and anything else as it is.
      def __hold__(value, types = nil)
        property_value = case value
                         when ::Array then Godot::Array.new(value)
                         when ::Hash then Godot::Dictionary.new(value)
                         else value
                         end
        types.nil? ? property_value : __type__(property_value, types)
      end

      # `container` typed as `types` say, each a type, an engine class and a
      # script: one for an array's elements, or a dictionary's keys and
      # values. One typed so already is the same container.
      def __type__(container, types)
        return container if container.is_typed && __read_types__(container) == types.inject(:+)
        return Godot::Array.new(container, *types.first) if types.size == 1

        Godot::Dictionary.new(container, *types[0], *types[1])
      end

      # The types `container` is typed with, as `__type__` takes them.
      def __read_types__(container)
        if container.is_a?(Godot::Array)
          return [container.get_typed_builtin, container.get_typed_class_name, container.get_typed_script]
        end

        [container.get_typed_key_builtin, container.get_typed_key_class_name, container.get_typed_key_script,
         container.get_typed_value_builtin, container.get_typed_value_class_name, container.get_typed_value_script]
      end

      # What each object's property starts from: the declared value, a typed
      # container's types beside it, or a tool button's method.
      def __read_default__(default, hint, types)
        return ButtonMethod.new(default) if hint == "tool_button"
        return ContainerDefault.new(default, types) unless types.nil?

        default
      end

      # Defines the writer of the exported property `name`, which holds what
      # it is given as `__hold__` says, typed as `types` say.
      def __define_export_writer__(name, types = nil)
        variable = :"@#{name}"
        define_method(:"#{name}=") do |value|
          instance_variable_set(variable, self.class.__send__(:__hold__, value, types))
        end
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

      # Takes back what the class exported, and notes each method the file at
      # `path` defined in it, before that file runs again and declares what
      # its source says now.
      def __withdraw__(path)
        @__exports__ = nil
        @__file_methods__ = []
        [self, singleton_class].each do |owner|
          (owner.instance_methods(false) + owner.private_instance_methods(false)).each do |name|
            definition = owner.instance_method(name)
            @__file_methods__ << [owner, name, definition] if definition.source_location&.first == path
          end
        end
      end

      # Removes each method the file's last run defined that its run now did
      # not define again, as a class loaded afresh would not have it.
      def __prune__
        (@__file_methods__ || []).each do |owner, name, definition|
          next unless owner.method_defined?(name) || owner.private_method_defined?(name)

          owner.__send__(:remove_method, name) if owner.instance_method(name) == definition
        end
        @__file_methods__ = nil
      end

      # Gives `object`, made before the class's file ran again, the value of
      # each property the class now exports that it holds nothing for.
      def __adopt__(object)
        __exports__.each do |name, value|
          next if object.instance_variable_defined?(:"@#{name}")

          object.instance_variable_set(:"@#{name}", __copy__(value, object))
        end
      end

      # The hint the keywords name and the value it is read with, as the
      # extension takes them. A property carries one hint, as a GDScript
      # variable carries one `@export_*` annotation, and `step:` belongs to
      # the range it steps through rather than being a hint of its own.
      def __read_hint__(keywords)
        step = keywords[:step]
        extra = keywords[:extra_hints]
        hints = keywords.keys.reject { |keyword| %i[step extra_hints].include?(keyword) }
        raise ArgumentError, %("#{hints.last}:" cannot be used with another hint.) if hints.size > 1

        hint = hints.first
        return ["range", __read_bounds__(keywords[:range], step, extra)] if hint == :range
        raise ArgumentError, %("step:" needs a "range:" to step through.) unless step.nil?
        raise ArgumentError, %("extra_hints:" needs a "range:" to add to.) unless extra.nil?
        return __read_type__(keywords[:type]) if hint == :type
        return ["none", nil] if hint.nil?

        [hint.to_s, __check_hint_value__(hint, keywords[hint])]
      end

      # The type `type:` names: a class, or the classes a typed container
      # holds, one in an array or a key's and a value's in a hash.
      def __read_type__(type)
        return ["type", __read_type_name__(type)] unless type.is_a?(::Array) || type.is_a?(::Hash)

        classes = type.is_a?(::Hash) ? type.keys + type.values : type
        unless (type.is_a?(::Array) ? 1 : 2) == classes.size && type.size == 1
          raise ArgumentError, %("type:" needs one class in an array or one pair in a hash, but #{type.inspect} was given instead.)
        end

        ["typed", classes.map { |element| __check_class__(:type, element) }]
      end

      # The value a hint is read with, a list where the hint takes the names
      # a value may take, which is refused where it is written otherwise.
      def __check_hint_value__(hint, value)
        if %i[enum flags].include?(hint) && !value.is_a?(::Array)
          raise ArgumentError, %("#{hint}:" needs a list of names, but #{value.inspect} was given instead.)
        end
        return value.map { |type| __check_class__(hint, type).to_s } if hint == :node_path && value.is_a?(::Array)
        if hint == :custom && !value.is_a?(::Array)
          raise ArgumentError, %("custom:" needs a hint and what it is read with, but #{value.inspect} was given instead.)
        end

        value
      end

      # The class a property names its type by, as the realm spells it.
      def __read_type_name__(type)
        __check_class__(:type, type).to_s
      end

      # The class the keyword `keyword` names a type by. Only a class names a
      # type, so anything else is refused where it is written.
      def __check_class__(keyword, type)
        raise ArgumentError, %("#{keyword}:" needs a class, but #{type.inspect} was given instead.) unless type.is_a?(::Module)

        type
      end

      # A range hint's bounds, the step included when one is declared. A
      # range leaving out its end names no highest value, so it is refused
      # where it is written.
      def __read_bounds__(range, step, extra = nil)
        unless range.is_a?(::Range) && !range.exclude_end?
          raise ArgumentError,
                %("range:" needs a range that includes its end, but #{range.inspect} was given instead.)
        end

        bounds = [range.begin, range.end]
        return step.nil? ? bounds : bounds << step if extra.nil?

        bounds + [step || 1] + (extra.is_a?(::Array) ? extra : [extra]).map(&:to_s)
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
      target, declared, bound, property = __resolve__(name)
      return super if target.nil?

      engine_class = self.class.__send__(:__engine_class__)
      if property
        engine_class.__send__(:define_method, name) { |*arguments| __call__(target, [property, *arguments]) } if declared
        __call__(target, [property, *args])
      elsif bound
        engine_class.__send__(:define_method, name, &engine_class.__send__(:__make_bound_body__, bound))
        engine_class.instance_method(name).bind_call(self, *args)
      else
        engine_class.__send__(:define_method, name) { |*arguments| __call__(target, arguments) } if declared
        __call__(target, args)
      end
    end

    # A freed engine object, or one not yet given its engine object, answers
    # by the engine class its Ruby class extends.
    def respond_to_missing?(name, include_private = false)
      !__resolve__(name).nil? || super
    rescue Godot::CallError, TypeError
      self.class.__send__(:__has_engine_member__, name) || super
    end

    # Two Ruby objects for one engine object are equal.
    def ==(other)
      other.is_a?(Godot::Object) && __instance_id__ == other.__send__(:__instance_id__)
    end
    alias eql? ==

    def hash
      __instance_id__.hash
    end

    # A copy of an engine object is made as Ruby makes one, standing for the
    # same engine object once initialize_copy reaches Godot::Object's; a node
    # class's object is the only one its node has, so it refuses.
    def dup
      __refuse_copy__("dup")
      super
    end

    def clone
      __refuse_copy__("clone")
      super
    end

    # The Ruby class, the engine object as Godot prints it, and the instance
    # variables as Ruby inspects them.
    def inspect
      stack = Godot::Object.__send__(:__inspection_stack__)
      head = "#<#{self.class} #{__label__}"
      return "#{head} ...>" if stack.any? { |object| object.equal?(self) }

      stack.push(self)
      variables = instance_variables.map { |name| "#{name}=#{instance_variable_get(name).inspect}" }
      variables.empty? ? "#{head}>" : "#{head} #{variables.join(", ")}>"
    ensure
      stack.pop if stack.last.equal?(self)
    end

    # Waits until `target` is emitted, in a method Godot runs where it may
    # wait, and answers what it was emitted with: nil, its one value, or its
    # values in an array, as GDScript's `await` does. `target` is a
    # Godot::Signal, an object followed by the name of its signal, or a
    # coroutine's state, whose end is awaited; any other value is answered
    # at once.
    def await(target, name = nil)
      target = Godot::Signal.new(target, name) unless name.nil?
      Coroutine.__send__(:wait_for, target)
    end

    private

    def __refuse_copy__(verb)
      raise TypeError, "can't #{verb} #{self.class}" unless self.class.instance_variable_get(:@engine_class) == true
    end

    # Runs the method `method`, which calls `await`, as Godot calls it.
    def __call_awaiting__(method, *args)
      Coroutine.run { __send__(method, *args) }
    end
  end

  # Where a method Godot runs may wait: each runs in a fiber of its own,
  # which yields as it awaits a signal and is resumed when the signal is
  # emitted, so Godot goes on while it waits, as with a GDScript coroutine.
  module Coroutine
    # The fibers running now that may wait, each with what it answers its
    # end through once it has waited. Only a fiber running is kept here; one
    # waiting is kept by its signal's connection alone, so it goes once the
    # object emitting the signal does.
    @fibers = {}

    # Runs `body` in a fiber of its own, answering what it returns when it
    # ends without waiting, or else a Signal emitted with what it returns
    # once it ends, which Godot and Ruby await as a GDScript coroutine's
    # state.
    def self.run(&body)
      fiber = Fiber.new(&body)
      ending = []
      answer = resume(fiber, ending)
      return answer unless fiber.alive?

      state = Godot::RefCounted.new
      state.add_user_signal("completed")
      ending << state
      Godot::Signal.new(state, :completed)
    end

    class << self
      private

      # Resumes `fiber` with `values`, emitting the state `ending` holds
      # once it has waited with what the fiber returns as it ends.
      def resume(fiber, ending, *values)
        @fibers[fiber] = ending
        answer = fiber.resume(*values)
        ending.first&.emit_signal(:completed, answer) unless fiber.alive?
        answer
      ensure
        @fibers.delete(fiber)
      end

      # Yields the fiber running now until `target` is emitted, answering
      # what it was emitted with; a value that is no signal is answered at
      # once. Only a fiber `run` made yields: any other Ruby is Godot's to
      # wait for. A yield mruby refuses, from under a method of its C, leaves
      # no connection behind.
      def wait_for(target)
        signal = read_signal(target)
        return target if signal.nil?

        fiber = Fiber.current
        ending = @fibers[fiber]
        if ending.nil?
          raise FiberError, "await waits only in a method Godot calls, or a block or method Godot calls back"
        end

        # The waiting fiber holds no Callable of its own connection, which
        # would hold the fiber back through its proc. A refused yield removes
        # the connection just made, the last the signal lists, since the
        # engine lists them in the order they were made.
        signal.connect(proc { |*args| resume(fiber, ending, args.size > 1 ? args : args.first) },
                       Godot::Object::CONNECT_ONE_SHOT)
        begin
          Fiber.yield
        rescue FiberError => e
          signal.disconnect(signal.get_connections.last["callable"])
          raise e
        end
      end

      # The signal awaiting `target` waits for: itself, or the end of a
      # coroutine's state; none for any other value.
      def read_signal(target)
        return target if target.is_a?(Godot::Signal)
        return unless target.is_a?(Godot::Object) && target.is_class("GDScriptFunctionState")

        Godot::Signal.new(target, :completed)
      end
    end
  end

  # A value of one of the engine's value types, such as Vector2 or Color:
  # built by the engine's constructors, answering the engine's members,
  # methods and operators, and never changed, since each side holds its own
  # copy.
  class Value
    # What an engine Array or Dictionary is: the engine's own, shared
    # wherever it is held, so a change to an element is made in it rather
    # than in a copy. A packed array a property answers is a copy, so it is
    # written back as a value is.
    module Reference
    end

    class << self
      # What `root.p1.p2 ... step = value` does, as GDScript assigns to a
      # member of a member: each value type the path reads is a copy, so the
      # copy with its member changed is written back into what it was read
      # from, up to the first link shared in place. A step is a member's name,
      # or the arguments of an index. Answers the root as it is now, which
      # changes only when it is itself a value type and every link was one.
      def __write_chain__(root, path, step, value)
        __write_links__(__read_links__(root, path), path, step, value)
      end

      # `__write_through__` for the commonest chain, one member read and one
      # written, as `position.y = 5`, with the writers' names made once where
      # the assignment is written.
      def __write_member__(owner, reader, writer, member, member_writer, value)
        link = owner.__send__(reader)
        if __copied__?(link)
          begin
            owner.__send__(writer, link.__send__(:__with_member__, member, value))
          rescue NoMethodError => e
            raise e if owner.respond_to?(writer, true)

            raise FrozenError, "can't modify the #{link.class} #{reader} answered: assign it to a variable first"
          end
        else
          link.__send__(member_writer, value)
        end
        value
      end

      # `__write_chain__` for a root no variable holds, answering `value`: a
      # root that would change is a copy a call answered, which nothing
      # would see changed.
      def __write_through__(root, path, step, value)
        unless __write_chain__(root, path, step, value).equal?(root)
          raise FrozenError, "can't modify #{root.class} a call answered: assign it to a variable first"
        end

        value
      end

      # `__write_chain__` for an operator assignment: the block makes the new
      # member from the current one, which `||=` keeps when it is truthy and
      # `&&=` when it is not.
      def __update_chain__(root, path, step, short = nil)
        links = __read_links__(root, path)
        current = __read_step__(links.last, step)
        return root if (short == :or && current) || (short == :and && !current)

        __write_links__(links, path, step, yield(current))
      end

      # Whether `link` is a value type's value, which is copied wherever it
      # is held, rather than anything shared in place.
      def __copied__?(link)
        Value === link && !(Reference === link)
      end

      # The values a multiple assignment takes from `values`, as Ruby splats
      # them.
      def __splat__(values)
        return values if ::Array === values
        return values.to_ary if values.respond_to?(:to_ary)

        [values]
      end

      def method_missing(name, *args, &block)
        answered = __call_static__(name, args)
        return super if answered.nil?

        answered.first
      end

      def const_missing(name)
        value = __constant_by_name__(name)
        return super if value.nil?

        const_set(name, value)
      end

      private :__has_value_type__, :__call_static__, :__constant_by_name__, :__engine_names__

      private

      # The root, then what each step of `path` reads from the link before.
      def __read_links__(root, path)
        path.each_with_object([root]) { |step, links| links << __read_step__(links.last, step) }
      end

      # `step` of the last of `links` set to `value`, each value type changed
      # written back into the link it was read from, up to the first one
      # shared in place; answers the root as it is now.
      def __write_links__(links, path, step, value)
        updated = __write_step__(links.last, step, value)
        index = path.size - 1
        while index >= 0 && __copied__?(links[index + 1])
          updated = __write_step__(links[index], path[index], updated)
          index -= 1
        end
        index.negative? && __copied__?(links.first) ? updated : links.first
      end

      def __read_step__(owner, step)
        ::Array === step ? owner.__send__(:[], *step) : owner.__send__(step)
      end

      # `owner` with `step` set to `value`: a copy for a value type, or
      # `owner` itself changed in place.
      def __write_step__(owner, step, value)
        if ::Array === step
          owner.__send__(:[]=, *step, value)
          owner
        elsif __copied__?(owner)
          owner.with(step => value)
        else
          writer = :"#{step}="
          unless owner.respond_to?(writer, true)
            raise FrozenError, "can't modify the #{value.class} #{step} answered: assign it to a variable first"
          end

          owner.__send__(writer, value)
          owner
        end
      end

      # What is defined on a value type's class is the engine's, never a file's.
      def const_added(name); end

      # Each member and method of the value type is the class's own from the
      # moment it is made, so it answers before a method of the name any
      # ancestor gains, as Integer's own methods answer before Numeric's. Its
      # body is bound at the first call; a name Godot::Value itself answers,
      # or the class or a module it includes answers in Ruby, stays theirs.
      def __define_engine_names__
        own = Value.instance_variable_get(:@own_methods) + __ruby_names__
        __engine_names__.each do |name|
          next if own.include?(name)

          define_method(name) do |*args, &block|
            raise NoMethodError, "undefined method '#{name}' for #{self.class}" unless __define_engine_name__(name)

            __send__(name, *args, &block)
          end
        end
      end

      # The names the class and the modules it includes define in Ruby.
      def __ruby_names__
        names = []
        ancestors.each do |ancestor|
          break if ancestor.equal?(Value)

          names.concat(ancestor.instance_methods(false)).concat(ancestor.private_instance_methods(false))
        end
        names
      end
    end

    # A Hash key matches only a value of its own type, as 4 never matches
    # 4.0, so keys of two types meet without the engine refusing to compare.
    def eql?(other)
      other.instance_of?(self.class) && self == other
    end

    def hash
      __hash__
    end

    def inspect
      "#<#{self.class} #{self}>"
    end

    # A copy of the value with each member named set to the value given, as
    # Ruby's Data#with gives one; the value itself never changes.
    def with(**members)
      members.reduce(self) { |value, (name, member)| value.__send__(:__with_member__, name, member) }
    end

    # A member or engine method the running engine has beyond those the class
    # was made with is defined on the class at its first call.
    def method_missing(name, *args, &block)
      raise FrozenError, "can't modify #{self.class}: build a new one instead" if name.to_s[-1] == "="
      return super unless __define_engine_name__(name)

      __send__(name, *args, &block)
    end

    def respond_to_missing?(name, include_private = false)
      !__resolve__(name).nil? || super
    end

    private

    # Defines the member or engine method `name` of the value's type on its
    # class, bound to the engine's getter or method, or answers false when
    # the type has none.
    def __define_engine_name__(name)
      kind, bound = __resolve__(name)
      case kind
      when :member
        if bound
          self.class.__send__(:define_method, name) { __get__(bound) }
        else
          self.class.__send__(:define_method, name) { __member_by_name__(name) }
        end
      when :method then self.class.__send__(:define_method, name) { |*arguments| __call__(name, arguments) }
      else return false
      end
      true
    end

    @own_methods = (instance_methods + private_instance_methods).freeze
  end

  class Value
    # What an engine array, an Array or a packed array, answers in Ruby: the
    # very one the engine holds, so a change shows wherever it is held.
    # Ruby's names for reading and writing an Array, and those Enumerable
    # and Comparable give, behave as Ruby's; the engine's other names
    # behave as the engine's.
    module Sequence
      include Comparable
      include Enumerable

      def each(&block)
        to_a.each(&block)
        self
      end

      def [](index, *length)
        return to_a[index, *length] unless index.is_a?(Integer) && length.empty?

        index += size if index < 0
        get(index) if index >= 0 && index < size
      end

      def []=(index, value)
        position = index < 0 ? index + size : index
        raise IndexError, "index #{index} too small for array; minimum: -#{size}" if position < 0

        resize(position + 1) if position >= size
        set(position, value)
      end

      def <<(value)
        push_back(value)
        self
      end

      def first(*count)
        count.empty? ? self[0] : to_a.first(*count)
      end

      def last(*count)
        count.empty? ? self[-1] : to_a.last(*count)
      end

      def length
        size
      end

      def empty?
        is_empty
      end

      def to_a
        __to_a__
      end

      def to_ary
        to_a
      end

      # Element by element, each of this array's elements asked first, so an
      # element the engine holds meets a Ruby value equal to it.
      def <=>(other)
        return unless other.is_a?(::Array) || other.is_a?(Sequence)

        other = other.to_a
        each_with_index do |element, index|
          return 1 if index >= other.size

          order = element <=> other[index]
          return order unless order == 0
        end
        size <=> other.size
      end

      # Godot::Value's, which Enumerable's would hide.
      def hash
        __hash__
      end
    end
  end

  # The engine's Array; Godot::Value::Sequence says how it answers.
  class Array < Value
    include Reference
    include Sequence

    __define_engine_names__
  end

  # The engine's packed arrays, each answering as Godot::Value::Sequence says.
  %i[PackedByteArray PackedInt32Array PackedInt64Array PackedFloat32Array PackedFloat64Array
     PackedStringArray PackedVector2Array PackedVector3Array PackedColorArray PackedVector4Array].each do |name|
    packed = const_set(name, Class.new(Value))
    packed.include(Value::Sequence)
    packed.__send__(:__define_engine_names__)
  end

  # The engine's Dictionary, the very one the engine holds, so a change
  # shows wherever it is held. Ruby's names for reading and writing a Hash,
  # and those Enumerable gives, behave as Ruby's; the engine's other names
  # behave as the engine's.
  class Dictionary < Value
    include Reference
    include Enumerable

    def each(&block)
      to_hash.each(&block)
      self
    end

    def [](key)
      get(key)
    end

    def []=(key, value)
      set(key, value)
    end

    def to_hash
      __to_hash__
    end

    # Entry by entry, each looked up here, where the engine takes a String
    # and a Symbol spelling one name for the same key.
    def ==(other)
      return false unless other.respond_to?(:to_hash)

      other = other.to_hash
      size == other.size && other.all? { |key, value| has(key) && self[key] == value }
    end

    # Godot::Value's, which Enumerable's would hide.
    def hash
      __hash__
    end

    __define_engine_names__
  end

  # Ruby's Array and Hash, compared with an engine container, ask the engine's
  # container, as CRuby's ask any object answering to_ary or to_hash.
  module ArrayEquality
    def ==(other)
      return super if other.is_a?(::Array) || !other.respond_to?(:to_ary)

      other == self
    end
  end
  ::Array.prepend(ArrayEquality)

  module HashEquality
    def ==(other)
      return super if other.is_a?(::Hash) || !other.respond_to?(:to_hash)

      other == self
    end
  end
  ::Hash.prepend(HashEquality)

  # What true and false are, as a class names a type, since Ruby has no class
  # of both; a signal's parameter of it is a bool.
  module Bool
  end
  ::TrueClass.include(Bool)
  ::FalseClass.include(Bool)

  # A value of any type, as a class names a type; a signal's parameter of it
  # is untyped, as one GDScript writes without a type.
  module Variant
  end
end

# A block or method Godot calls back runs where it may await, as a GDScript
# lambda may.
[::Proc, ::Method].each do |callable|
  callable.class_eval do
    private

    def __call_awaiting__(*args)
      Godot::Coroutine.run { call(*args) }
    end
  end
end
