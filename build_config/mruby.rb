# mruby's version is pinned by the beni gem; a lockfile beside this config
# would only repeat it.
MRuby::Lockfile.disable

# The fewest gems the extension needs; anything that reaches the host (IO,
# sockets, directories) stays out, since Ruby reaches the host through Godot's
# API.
GEMS = %w[mruby-compiler mruby-fiber mruby-math mruby-metaprog mruby-method mruby-random mruby-sprintf].freeze

# What every build shares, as each goes into the same shipped library. That
# library is a shared one, so the archive's code is compiled
# position-independent; MSVC has no such flag, its code already is.
def extension_build(conf)
  GEMS.each { |name| conf.gem core: name }
  conf.compilers.each { |cc| cc.flags << "-fPIC" } unless conf.primary_toolchain == "visualcpp"
end

MRuby::Build.new do |conf|
  # load specific toolchain settings
  conf.toolchain

  # Use mrbgems
  # conf.gem 'examples/mrbgems/ruby_extension_example'
  # conf.gem 'examples/mrbgems/c_extension_example' do |g|
  #   g.cc.flags << '-g' # append cflags in this gem
  # end
  # conf.gem 'examples/mrbgems/c_and_ruby_extension_example'
  # conf.gem :core => 'mruby-eval'
  # conf.gem :mgem => 'mruby-onig-regexp'
  # conf.gem :github => 'mattn/mruby-onig-regexp'
  # conf.gem :git => 'git@github.com:mattn/mruby-onig-regexp.git', :branch => 'master', :options => '-v'

  extension_build(conf)

  # C compiler settings
  # conf.cc do |cc|
  #   cc.command = ENV['CC'] || 'gcc'
  #   cc.flags = [ENV['CFLAGS'] || %w()]
  #   cc.include_paths = ["#{root}/include"]
  #   cc.defines = %w()
  #   cc.option_include_path = %q[-I"%s"]
  #   cc.option_define = '-D%s'
  #   cc.compile_options = %Q[%{flags} -MMD -o "%{outfile}" -c "%{infile}"]
  # end

  # mrbc settings
  # conf.mrbc do |mrbc|
  #   mrbc.compile_options = "-g -B%{funcname} -o-" # The -g option is required for line numbers
  # end

  # Linker settings
  # conf.linker do |linker|
  #   linker.command = ENV['LD'] || 'gcc'
  #   linker.flags = [ENV['LDFLAGS'] || []]
  #   linker.flags_before_libraries = []
  #   linker.libraries = %w()
  #   linker.flags_after_libraries = []
  #   linker.library_paths = []
  #   linker.option_library = '-l%s'
  #   linker.option_library_path = '-L%s'
  #   linker.link_options = %Q[%{flags} -o "%{outfile}" %{objs} %{libs}]
  # end

  # Archiver settings
  # conf.archiver do |archiver|
  #   archiver.command = ENV['AR'] || 'ar'
  #   archiver.archive_options = 'rs "%{outfile}" %{objs}'
  # end

  # Parser generator settings
  # conf.yacc do |yacc|
  #   yacc.command = ENV['YACC'] || 'bison'
  #   yacc.compile_options = %q[-o "%{outfile}" "%{infile}"]
  # end

  # gperf settings
  # conf.gperf do |gperf|
  #   gperf.command = 'gperf'
  #   gperf.compile_options = %q[-L ANSI-C -C -j1 -i 1 -o -t -N mrb_reserved_word -k"1,3,$" "%{infile}" > "%{outfile}"]
  # end

  # file extensions
  # conf.exts do |exts|
  #   exts.object = '.o'
  #   exts.executable = '' # '.exe' if Windows
  #   exts.library = '.a'
  # end

  # file separator
  # conf.file_separator = '/'

  # change library directory name from the default "lib" if necessary
  # conf.libdir_name = 'lib64'

  # Turn on `enable_debug` for better debugging
  # conf.enable_debug
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
