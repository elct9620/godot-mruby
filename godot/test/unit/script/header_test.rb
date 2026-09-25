class HeaderTest < Minitest::Test
  DIRECTORY = "res://test/unit/script/header".freeze
  RANGE = 1
  GROUP = 64

  # @behavior RS-008
  def test_a_node_script_reports_the_engine_class_it_extends_before_its_file_runs
    assert_equal :Node2D, script(:reported).get_instance_base_type
    refute ran?(:Reported)
  end

  # @behavior RS-009
  def test_a_node_script_reports_the_methods_its_class_defines_before_its_file_runs
    with_node(:reported) { |node| assert node.has_method(:_ready) }
    refute ran?(:Reported)
  end

  # @behavior RS-018
  def test_a_node_script_reports_that_its_class_is_a_tool_before_its_file_runs
    assert script(:marked).is_tool
    refute ran?(:Marked)
  end

  # @behavior RS-019
  def test_a_node_script_reports_that_its_class_is_abstract_before_its_file_runs
    assert script(:marked).is_abstract
    refute ran?(:Marked)
  end

  # @behavior RS-021
  def test_a_node_script_reports_the_engine_class_it_extends_through_other_files_before_its_file_runs
    assert_equal :Node2D, script(:boss).get_instance_base_type
    refute ran?(:Boss)
  end

  # @behavior RS-022
  def test_a_node_script_reports_the_script_of_the_class_it_extends
    assert_equal "#{DIRECTORY}/enemy.rb", script(:boss).get_base_script.resource_path
    refute ran?(:Boss)
  end

  # @behavior RS-038
  def test_a_node_script_reports_the_signals_its_class_declares_before_its_file_runs
    with_node(:siren) { |node| assert node.has_signal(:wailed) }
    refute ran?(:Siren)
  end

  # @behavior RS-039
  def test_a_scenes_connection_to_a_declared_signal_is_made_before_its_file_runs
    listener = autofree(Godot::ResourceLoader.load("#{DIRECTORY}/siren.tscn").instantiate)

    assert listener.get_node("Siren").has_connections(:wailed)
    refute ran?(:Siren)
  end

  # @behavior RS-041
  def test_a_node_script_lists_an_exported_property_before_its_file_runs
    with_node(:siren) { |node| assert_includes names(node.get_property_list), "volume" }
    refute ran?(:Siren)
  end

  # @behavior RS-043
  def test_a_node_lists_the_methods_its_class_defines
    with_node(:siren) { |node| assert_includes names(node.get_method_list), "wail" }
    refute ran?(:Siren)
  end

  # @behavior RS-044
  def test_a_node_lists_the_methods_its_class_inherits_from_another_file
    with_node(:boss) { |node| assert_includes names(node.get_method_list), "charge" }
    refute ran?(:Boss)
  end

  # @behavior RS-050
  def test_a_node_script_lists_an_exported_propertys_hint_before_its_file_runs
    angle = script(:dial).get_script_property_list.find { |property| property["name"] == "angle" }

    assert_equal [RANGE, "0,360"], [angle["hint"], angle["hint_string"]]
    refute ran?(:Dial)
  end

  # @behavior RS-051
  def test_a_node_script_lists_the_headings_its_class_writes_before_its_file_runs
    listed = names(script(:dial).get_script_property_list)

    assert_equal %w[Aim angle], listed.last(2)
    refute ran?(:Dial)
  end

  private

  def script(name)
    Godot::ResourceLoader.load("#{DIRECTORY}/#{name}.rb")
  end

  # A node with the script of that name.
  def with_node(name)
    node = autofree(Godot::Node2D.new)
    node.set_script(script(name))
    yield node
  end

  # Whether the file defining the class of that name has run.
  def ran?(name)
    Unit::Script::Header.const_defined?(name)
  end

  def names(described)
    described.map { |entry| entry["name"] }
  end
end
