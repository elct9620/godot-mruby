# What a class has of its own, the Ruby way: a module prepended to Module
# tells the realm of every method a module gains and every module one takes
# in after Ruby's own hooks, and answers the modules a class takes in.
# `__record_change__` is the extension's.
Module.prepend(Module.new do
  # A method a class whose file has run gains, or a module it takes in, is
  # one Godot may call, so the realm reads the class's methods again.
  def method_added(name)
    __record_change__(self)
    super
  end

  def included(base)
    __record_change__(base)
    super
  end

  def prepended(base)
    __record_change__(base)
    super
  end

  # A method it loses is one Godot may no longer call.
  def method_removed(name)
    __record_change__(self)
    super
  end

  def method_undefined(name)
    __record_change__(self)
    super
  end
  private :method_added, :included, :prepended, :method_removed, :method_undefined

  # mruby defines an attribute's methods without calling method_added.
  %i[attr_reader attr_writer attr_accessor attr].each do |declare|
    define_method(declare) do |*names|
      defined = super(*names)
      __record_change__(self)
      defined
    end
  end

  # The class itself and the modules it takes in, up to its superclass.
  def __own_ancestors__
    stop = is_a?(Class) ? superclass : nil
    taken = []
    ancestors.each do |taken_in|
      break if taken_in.equal?(stop)

      taken << taken_in
    end
    taken
  end
  private :__own_ancestors__
end)
