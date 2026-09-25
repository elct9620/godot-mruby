# frozen_string_literal: true

module Godot
  # Creates a Ruby script from a template in the editor's script dialog, in a
  # copy of the integration-test project, and reads the file it made, then
  # plays a scene holding it. Part of Godot.verify!.
  module Template
    PROBE = File.join("integration", "template", "probe")
    FRAMES = "120"
    MADE = File.join("enemies", "ships", "hero_ship.rb")
    # The namespaces the path spells around the class, each a level deeper by
    # whatever the editor indents with.
    OPENED = /\Amodule Enemies\n(\s+)module Ships\n\1\1class HeroShip < Godot::CharacterBody2D\n/
    CLOSED = /\n(\s+)\1end\n\1end\nend\n\z/
    # The project's own template, listed by the name its meta line gives.
    LISTED = "template listed: Node: Friendly Hello"
    SCENE = "hero.tscn"
    SCENE_SOURCE = <<~TSCN.freeze
      [gd_scene load_steps=2 format=3]

      [ext_resource type="Script" path="res://#{MADE}" id="1"]

      [node name="Hero" type="CharacterBody2D"]
      script = ExtResource("1")
    TSCN

    module_function

    def verify!(project)
      Godot.with_probe(project, PROBE) do |copy|
        output, = Godot.run_editor_frames(copy, FRAMES)
        verify_listed!(output)
        verify_made!(copy, output)
      end
    end

    # @behavior RY-008
    def verify_listed!(output)
      return if output.include?(LISTED)

      raise "The script dialog did not list the project's template as #{LISTED}:\n#{output}"
    end

    # A template saved under a namespace opens the namespaces its path
    # spells, and the class it made runs as the node's script.
    # @behavior RY-007
    def verify_made!(copy, output)
      made = File.join(copy, MADE)
      raise "The script dialog made no #{MADE}:\n#{output}" unless File.exist?(made)

      verify_opened!(File.read(made))
      verify_played!(copy)
    end

    def verify_opened!(source)
      return if source.match?(OPENED) && source.match?(CLOSED) && source.include?("move_and_slide")

      raise "#{MADE} was not made inside its namespaces:\n#{source}"
    end

    def verify_played!(copy)
      File.write(File.join(copy, SCENE), SCENE_SOURCE)
      output, = Godot.run_scene(copy, "res://#{SCENE}", "--quit-after", "10")
      return if output.lines.grep(FAILED).empty?

      raise "A scene holding #{MADE} did not run cleanly:\n#{output}"
    end
  end
end
