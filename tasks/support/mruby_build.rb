# frozen_string_literal: true

# The mruby builds beni makes for the extension's libraries, as
# build_config/mruby.rb names them: each architecture's carries the debug
# hook, and beside it is the one the release export template's library links,
# named after that library.
module MrubyBuild
  HOOKLESS_KIND = "template"

  module_function

  # Every build on `bases`, the architectures the libraries are built for.
  def names(bases)
    bases.flat_map { |base| [base, name(base, HOOKLESS_KIND)] }
  end

  # The build the library of `kind` links on `base`.
  def name(base, kind)
    kind == HOOKLESS_KIND ? "#{base}-#{kind}" : base
  end
end
