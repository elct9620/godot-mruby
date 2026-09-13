# Ruby's own const_missing, asked first of the class index: the first time
# Ruby misses a constant a file under res:// names, that file runs, and every
# other miss is left to Ruby. `__load_by_name__` is the extension's.
Module.prepend(Module.new do
  def const_missing(name)
    found = __load_by_name__(name)
    found ? found.first : super
  end
end)
