# frozen_string_literal: true

require "fileutils"
require "tmpdir"

module Godot
  module Script
    # Runs a copy of the integration-test project that lists a node script
    # as a singleton node, with a main scene whose GDScript reaches it by
    # name, and reads what both print. Part of Script.verify!.
    module SingletonNode
      # The singleton node's name and script, and the project setting
      # listing it, which Godot spells as its own section.
      NAME = "Concierge"
      SCRIPT = "res://integration/script/singleton_node/steward.rb"
      OVERRIDE = %([autoload]\n\n#{NAME}="*#{SCRIPT}"\n).freeze
      # The main scene the copy gets, and its GDScript calling the singleton node.
      SCENE = "res://visitor.tscn"
      FILES = {
        "override.cfg" => OVERRIDE,
        "visitor.tscn" => %([gd_scene format=3]\n\n[ext_resource type="Script" path="res://visitor.gd" id="1"]\n\n) +
                          %([node name="Visitor" type="Node"]\nscript = ExtResource("1")\n),
        "visitor.gd" => <<~GDSCRIPT
          extends Node


          func _ready() -> void:
          \tprint("visitor heard ", #{NAME}.call("greeting"))
        GDSCRIPT
      }.freeze
      READY_LINE = "steward ready at /root/#{NAME}".freeze
      HEARD_LINE = "visitor heard welcome"

      module_function

      def verify!(project)
        Dir.mktmpdir do |dir|
          copy = File.join(dir, "project")
          FileUtils.cp_r(project, copy)
          FILES.each { |name, source| File.write(File.join(copy, name), source) }
          output, status = Godot.run_scene(copy, SCENE, "--quit-after", "3")
          raise "The singleton node's scene did not run:\n#{output}" unless status.success?

          lines = output.lines.map(&:chomp)
          verify_ready_first!(lines, output)
          verify_reached_by_name!(lines, output)
        end
      end

      # @behavior RS-079
      def verify_ready_first!(lines, output)
        ready = lines.index(READY_LINE)
        heard = lines.index(HEARD_LINE)
        return if ready && heard && ready < heard

        raise "The singleton node #{NAME} was not ready at the root before the main scene:\n#{output}"
      end

      # @behavior RS-080
      def verify_reached_by_name!(lines, output)
        return if lines.include?(HEARD_LINE)

        raise "GDScript did not reach the singleton node #{NAME} by its name:\n#{output}"
      end
    end
  end
end
