class TypingTest < Minitest::Test
  PATH = "res://test/unit/script/tuned.rb".freeze
  RANGE = 1
  CHANGED = <<~RUBY.freeze
    module Unit
      module Script
        class Tuned < Godot::Node2D
          TREBLE = 3
          NOTES = %i[pitch].freeze

          signal :"\#{:str}ummed"
          signal :sung, *NOTES
          signal :rang, :times

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
    volume = members.find { |property| property["name"] == "volume" }

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

  # @behavior RS-062
  def test_a_node_script_lists_a_signal_its_changed_source_declares_before_the_file_runs_again
    rang = signals.find { |signal| signal["name"] == "rang" }

    assert_equal ["times"], rang["args"].map { |argument| argument["name"] }
  end

  # @behavior RS-063
  def test_a_node_script_drops_a_signal_its_changed_source_no_longer_declares
    refute_includes signal_names, "hummed"
  end

  # @behavior RS-064
  def test_a_node_script_keeps_a_signal_no_signal_call_of_its_source_names_while_its_source_changes
    assert_includes signal_names, "strummed"
  end

  # @behavior RS-065
  def test_a_node_script_keeps_what_its_run_declared_for_a_signal_whose_parameters_its_changed_source_does_not_write_out
    sung = signals.find { |signal| signal["name"] == "sung" }

    assert_equal ["pitch"], sung["args"].map { |argument| argument["name"] }
  end

  private

  def script
    Godot::ResourceLoader.load(PATH)
  end

  def members
    script.get_script_property_list
  end

  def signals
    script.get_script_signal_list
  end

  def signal_names
    signals.map { |signal| signal["name"] }
  end

  def names
    members.map { |property| property["name"] }
  end
end
