class TypingTest < Minitest::Test
  PATH = "res://test/unit/script/tuned.rb".freeze
  RANGE = 1
  CHANGED = <<~RUBY.freeze
    module Unit
      module Script
        class Tuned < Godot::Node2D
          TREBLE = 3

          export :treble, TREBLE
          export :"\#{:mid}dle", 2
          export :volume, 5, range: 0..10
        end
      end
    end
  RUBY

  def setup
    Unit::Script::Tuned
    @source = script.source_code
    script.source_code = CHANGED
  end

  def teardown
    script.source_code = @source
  end

  # @behavior RS-052
  def test_a_node_script_lists_what_its_changed_source_exports_before_the_file_runs_again
    volume = listed.find { |property| property["name"] == "volume" }

    assert_equal [RANGE, "0,10"], [volume["hint"], volume["hint_string"]]
  end

  # @behavior RS-053
  def test_a_node_script_keeps_what_its_run_declared_for_an_export_its_changed_source_does_not_write_out
    assert_includes names, "treble"
  end

  # @behavior RS-054
  def test_a_node_script_drops_an_export_its_changed_source_no_longer_writes
    refute_includes names, "bass"
  end

  # @behavior RS-055
  def test_a_node_script_keeps_an_export_no_export_call_of_its_source_names_while_its_source_changes
    assert_includes names, "middle"
  end

  private

  def script
    Godot::ResourceLoader.load(PATH)
  end

  def listed
    script.get_script_property_list
  end

  def names
    listed.map { |property| property["name"] }
  end
end
