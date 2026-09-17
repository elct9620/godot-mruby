module Loader
  class Shading
  end

  # Opens the module of shades/, which has no file of its own, then raises.
  module Shades
  end

  raise "shading.rb fails after opening Loader::Shades"
end
