# In the editor, selects the beacon so the Signals dock lists its signals,
# and once the dock has had frames to fill, prints each signal row it shows.
module Integration
  module Script
    module Signals
      class Signalman < Godot::Node
        tool

        # The frame the beacon is selected at, once the editor has opened the
        # scene, which selects its own node, and the frame the dock is read at,
        # once the editor's realm has run the beacon's file.
        SELECTION_FRAME = 10
        INSPECTION_FRAME = 30

        # A person's editor is left alone: the probe changes the selection.
        def _ready
          @frames = 0 if Godot::Engine.is_editor_hint && Godot::DisplayServer.get_name == "headless"
        end

        def _process(_delta)
          return if @frames.nil?

          @frames += 1
          Godot::EditorInterface.edit_node(get_node("Beacon")) if @frames == SELECTION_FRAME
          return unless @frames == INSPECTION_FRAME

          dock = Godot::EditorInterface.get_base_control.find_children("*", "ConnectionsDock", true, false).first
          tree = dock&.find_children("*", "Tree", true, false)&.first
          print_rows(tree&.get_root)
        end

        private

        def print_rows(item)
          return if item.nil?

          puts "signal row: #{item.get_text(0)}" unless item.get_text(0).empty?
          item.get_children.each { |child| print_rows(child) }
        end
      end
    end
  end
end
