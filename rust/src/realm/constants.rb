# Loading by name, the Ruby way: a module prepended to Module asks the class
# index before Ruby's own const_missing, and tells the realm of every constant
# a running file creates before Ruby's own const_added. `__load_by_name__` and
# `__record_constant__` are the extension's.
Module.prepend(Module.new do
  # The first time Ruby misses a constant a file under res:// names, that file
  # runs; every other miss is left to Ruby.
  def const_missing(name)
    found = __load_by_name__(name)
    found ? found.first : super
  end

  # A file that raises takes away what it created, so the realm has to know,
  # and a namespace's own constant the new one would hide loads now. Private,
  # as Ruby's own hook is.
  def const_added(name)
    __record_constant__(name)
    super
  end
  private :const_added
end)
