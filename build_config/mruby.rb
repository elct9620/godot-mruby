# mruby's version is pinned by the beni gem; a lockfile beside this config
# would only repeat it.
MRuby::Lockfile.disable

# The fewest gems a game needs; anything that reaches the host (IO,
# sockets, directories) stays out, since Ruby reaches the host through Godot's
# API.
GEMS = %w[
  mruby-array-ext mruby-compar-ext mruby-compiler mruby-data mruby-enum-ext mruby-enumerator mruby-fiber
  mruby-hash-ext mruby-kernel-ext mruby-math mruby-metaprog mruby-method mruby-numeric-ext mruby-random
  mruby-range-ext mruby-set mruby-sprintf mruby-string-ext mruby-struct
].freeze

# What every build shares, as each goes into the same shipped library. That
# library is a shared one, so the archive's code is compiled
# position-independent; MSVC has no such flag, its code already is. On MSVC
# the library links the C runtime statically (.cargo/config.toml), and the
# archive follows with /MT, or its code would still import the runtime DLL.
def extension_build(conf)
  GEMS.each { |name| conf.gem core: name }
  if conf.primary_toolchain == "visualcpp"
    conf.compilers.each { |cc| cc.flags = cc.flags.flatten.map { |flag| flag == "/MD" ? "/MT" : flag } }
  else
    conf.compilers.each { |cc| cc.flags << "-fPIC" }
  end
end

# The editor's library and the debug export template's carry mruby's debug
# hook, which a debugger stops at breakpoints from: every instruction asks
# whether one is installed. The release export template's library links a
# build without it, named after that library, so a shipped game pays nothing.
def debug_hook(conf)
  conf.compilers.each { |cc| cc.defines << "MRB_USE_DEBUG_HOOK" }
end

MRuby::Build.new do |conf|
  conf.toolchain
  extension_build(conf)
  debug_hook(conf)
end

MRuby::Build.new("host-template") do |conf|
  conf.toolchain
  extension_build(conf)
end

# The x86_64 half of the universal macOS library. The build machine is Apple
# Silicon, whose compiler targets either architecture, so this is the host
# build again for x86_64, named after the cargo target that links it.
if RUBY_PLATFORM.include?("darwin")
  def x86_64_build(conf)
    conf.toolchain
    conf.cc.flags << "-arch x86_64"
    conf.linker.flags << "-arch x86_64"
    extension_build(conf)
  end

  MRuby::CrossBuild.new("x86_64-apple-darwin") do |conf|
    x86_64_build(conf)
    debug_hook(conf)
  end

  MRuby::CrossBuild.new("x86_64-apple-darwin-template") do |conf|
    x86_64_build(conf)
  end
end
