# mruby's version is pinned by the beni gem; a lockfile beside this config
# would only repeat it.
MRuby::Lockfile.disable

# The fewest gems the extension needs; anything that reaches the host (IO,
# sockets, directories) stays out, since Ruby reaches the host through Godot's
# API.
GEMS = %w[
  mruby-compiler mruby-fiber mruby-math mruby-metaprog mruby-method mruby-numeric-ext mruby-random mruby-sprintf
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

MRuby::Build.new do |conf|
  conf.toolchain
  extension_build(conf)
end

# The x86_64 half of the universal macOS library. The build machine is Apple
# Silicon, whose compiler targets either architecture, so this is the host
# build again for x86_64, named after the cargo target that links it.
if RUBY_PLATFORM.include?("darwin")
  MRuby::CrossBuild.new("x86_64-apple-darwin") do |conf|
    conf.toolchain
    conf.cc.flags << "-arch x86_64"
    conf.linker.flags << "-arch x86_64"

    extension_build(conf)
  end
end
