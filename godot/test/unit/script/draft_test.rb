class DraftTest < Minitest::Test
  GIVEN_SOURCE = <<~RUBY.freeze
    module Unit
      module Script
        class Draft < Godot::Node
          def source = "the source Godot holds"
        end
      end
    end
  RUBY

  # @behavior RS-005
  def test_a_script_runs_the_source_godot_holds_for_it
    script = Godot::ResourceLoader.load("res://test/unit/script/draft.rb")
    script.source_code = GIVEN_SOURCE
    node = autofree(Godot::Node.new)
    node.set_script(script)

    assert_equal "the source Godot holds", node.call(:source)
  end
end
