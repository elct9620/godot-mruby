# frozen_string_literal: true

require "fileutils"

require_relative "../godot"

module Bench
  # Writes a project of generated library files, spread over directories, and
  # node scripts extending one base, beside the addon and the scene in
  # tasks/bench that times them, for Bench and Godot::Loader::Scaling.
  module Project
    TEMPLATE = File.expand_path("../../bench", __dir__)
    # Library files are spread over this many directories, and a node script
    # is written for every this many library files.
    DIRECTORIES = 10
    FILES_PER_SCRIPT = 10

    module_function

    # Writes the project at `project`, taking the addon from the project at
    # `source`.
    def generate(project, files, source = Godot::PROJECT)
      install(project, source)
      names = Array.new(files) { |index| write_item(project, index) }
      scripts = Array.new(files / FILES_PER_SCRIPT) { |index| write_unit(project, index) }
      write(project, "manifest.rb",
            "module Manifest\n  NAMES = #{names.inspect}\n  SCRIPTS = #{scripts.inspect}\nend\n")
    end

    # Copies the scene that times the project, and the addon of `source` with
    # the extension list that loads it.
    def install(project, source)
      FileUtils.cp_r(File.join(TEMPLATE, "."), project)
      FileUtils.mkdir_p([File.join(project, ".godot"), File.join(project, "addons")])
      FileUtils.cp_r(File.join(source, "addons", "godot_mruby"), File.join(project, "addons"))
      FileUtils.cp(File.join(source, Godot::EXTENSION_LIST), File.join(project, Godot::EXTENSION_LIST))
    end

    # Writes the library file at `index`, answering the names it is reached by.
    def write_item(project, index)
      directory = "D#{index % DIRECTORIES}"
      name = "Item#{index}"
      write(project, "lib/#{directory.downcase}/item#{index}.rb",
            "module Lib\n  module #{directory}\n    class #{name}\n    end\n  end\nend\n")
      ["Lib", directory, name]
    end

    # Writes the node script at `index`, answering its path.
    def write_unit(project, index)
      path = "lib/units/unit#{index}.rb"
      write(project, path, "module Lib\n  module Units\n    class Unit#{index} < Base\n    end\n  end\nend\n")
      "res://#{path}"
    end

    def write(project, path, source)
      path = File.join(project, path)
      FileUtils.mkdir_p(File.dirname(path))
      File.write(path, source)
    end
  end
end
