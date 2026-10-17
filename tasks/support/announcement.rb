# frozen_string_literal: true

module Godot
  # Adds a node script sharing an announced name while the editor runs, in a
  # copy of the integration-test project, and reads how the editor lists the
  # name after it scans. Part of Godot.verify!.
  module Announcement
    PROBE = File.join("integration", "announcement", "rescan")
    FRAMES = "200"
    # What the probe prints of the class list before the namesake came and
    # once the scan took it in.
    RESCAN_LINES = ["Boss listed before the file came: true", "Boss listed after the scan: false"].freeze

    module_function

    def verify!(project)
      Godot.copy_probe_project(project, PROBE) do |copy|
        output, = Godot.run_editor_frames(copy, FRAMES)
        verify_rescanned!(output)
      end
    end

    # @behavior RN-017
    def verify_rescanned!(output)
      return if RESCAN_LINES.all? { |line| output.include?(line) }

      raise "The editor kept listing boss.rb by its name once a namesake was added:\n#{output}"
    end
  end
end
