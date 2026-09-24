# frozen_string_literal: true

require "fileutils"
require "tmpdir"

module Godot
  module Script
    # Scans the integration-test project in the editor and reads how its node
    # scripts are announced: the class list the editor writes, and what it warns
    # of. Part of Script.verify!.
    module Announcement
      # The class list the editor writes as it scans the project, and the entry
      # it lists boss.rb by: its base announced from grunt.rb, and its icon from
      # beside the file.
      CLASS_LIST = File.join(".godot", "global_script_class_cache.cfg")
      BOSS_ENTRY = [
        '"base": &"Grunt"', '"class": &"Boss"', '"icon": "res://integration/script/inherit/boss.svg"'
      ].freeze
      TWINS = "res://integration/script/announce"
      TWINS_WARNING = "WARNING: #{TWINS}/left/twin.rb and #{TWINS}/right/twin.rb " \
                      "define node scripts named Twin, so none is listed by that name".freeze
      # The twins are the project's only node scripts meant to share a name, so
      # any other pair sharing one is a file that took a name already spelled.
      SHARED_NAME = /define node scripts named (?<name>\w+), so none is listed/
      # A node script the editor cannot read, and what a scan must never print
      # while the language answers for it.
      UNREADABLE = "unreadable.rb"
      UNREADABLE_SOURCE = "class Unreadable < Godot::Node\nend\n"
      PANIC = "already bound"

      module_function

      def verify!(project)
        output, status = Godot.run_editor(project)
        raise "The editor did not scan the project:\n#{output}" unless status.success?

        verify_listed!(project)
        verify_twins_warned!(output)
        verify_names_apart!(output)
        verify_unreadable_skipped!(project)
      end

      # @behavior RS-025
      def verify_listed!(project)
        entries = File.read(File.join(project, CLASS_LIST)).split("}, {")
        return if entries.any? { |entry| BOSS_ENTRY.all? { |line| entry.include?(line) } }

        raise "The editor did not list boss.rb by its announcement #{BOSS_ENTRY}:\n#{entries.join("}, {")}"
      end

      # @behavior RS-029
      def verify_unreadable_skipped!(project)
        output, status = scan_with_unreadable(project)
        return if output.nil?
        return if status.success? && !output.include?(PANIC) && !output.include?("res://#{UNREADABLE}")

        raise "The editor did not pass over #{UNREADABLE} quietly:\n#{output}"
      end

      # Scans a copy of the project holding a node script nobody may read, and
      # answers nothing where the system still lets its owner read it, as
      # Windows does.
      def scan_with_unreadable(project)
        Dir.mktmpdir do |dir|
          copy = File.join(dir, "project")
          FileUtils.cp_r(project, copy)
          FileUtils.rm_rf(File.join(copy, ".godot", "editor"))
          unreadable = File.join(copy, UNREADABLE)
          File.write(unreadable, UNREADABLE_SOURCE)
          File.chmod(0o000, unreadable)
          File.readable?(unreadable) ? nil : Godot.run_editor(copy)
        end
      end

      # Every node script but the twins has a name of its own, so a fixture
      # never takes one another file already spells.
      def verify_names_apart!(output)
        shared = output.scan(SHARED_NAME).flatten.uniq - ["Twin"]
        return if shared.empty?

        raise "Node scripts other than the twins share a name #{shared}:\n#{output}"
      end

      # @behavior RS-026
      def verify_twins_warned!(output)
        return if output.include?(TWINS_WARNING)

        raise "The editor did not warn that the twins share a name:\n#{output}"
      end
    end
  end
end
