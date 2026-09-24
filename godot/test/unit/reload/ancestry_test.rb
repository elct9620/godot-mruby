class AncestryTest < Minitest::Test
  DIRECTORY = "res://test/unit/reload".freeze

  SHIFTED = <<~RUBY.freeze
    module Unit
      module Reload
        class Shifting < Godot::Node2D
        end
      end
    end
  RUBY

  # @behavior RF-001
  def test_a_node_script_reports_the_engine_class_the_file_it_extends_now_reaches
    shifted = script(:shifted)
    shifted.get_instance_base_type
    changing(script(:shifting), SHIFTED) do
      assert_equal :Node2D, shifted.get_instance_base_type
    end
  end

  private

  def script(name)
    Godot::ResourceLoader.load("#{DIRECTORY}/#{name}.rb")
  end

  # Runs the block while Godot holds `source` for `script`, then gives back
  # what it held.
  def changing(script, source)
    held = script.source_code
    script.source_code = source
    yield
  ensure
    script.source_code = held
  end
end
