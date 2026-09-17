# frozen_string_literal: true

module Godot
  # Scans the integration-test project in the editor and reads how its node
  # scripts are announced: the class list the editor writes, and what it warns
  # of. Part of Godot.verify!.
  module Announcement
    # The class list the editor writes as it scans the project, and the entry
    # it lists boss.rb by: its base announced from enemy.rb, and its icon from
    # beside the file.
    CLASS_LIST = File.join(".godot", "global_script_class_cache.cfg")
    BOSS_ENTRY = [
      '"base": &"Enemy"', '"class": &"Boss"', '"icon": "res://verify/script/inherit/boss.svg"'
    ].freeze
    TWINS = "res://verify/script/announce"
    TWINS_WARNING = "WARNING: #{TWINS}/left/twin.rb and #{TWINS}/right/twin.rb " \
                    "define node scripts named Twin, so none is listed by that name".freeze

    module_function

    def verify!(project)
      output, status = Godot.run_editor(project)
      raise "The editor did not scan the project:\n#{output}" unless status.success?

      verify_listed!(project)
      verify_twins_warned!(output)
    end

    # @behavior RS-025
    def verify_listed!(project)
      entries = File.read(File.join(project, CLASS_LIST)).split("}, {")
      return if entries.any? { |entry| BOSS_ENTRY.all? { |line| entry.include?(line) } }

      raise "The editor did not list boss.rb by its announcement #{BOSS_ENTRY}:\n#{entries.join("}, {")}"
    end

    # @behavior RS-026
    def verify_twins_warned!(output)
      return if output.include?(TWINS_WARNING)

      raise "The editor did not warn that the twins share a name:\n#{output}"
    end
  end
end
