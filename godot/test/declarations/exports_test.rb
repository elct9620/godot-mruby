class ExportsTest < Minitest::Test
  # @behavior RD-011
  def test_an_exported_property_is_one_the_script_has
    turret = Loader::Turret.new
    script = Godot::ResourceLoader.load("res://loader/turret.rb")

    names = script.get_script_property_list.map { |property| property["name"] }

    assert_includes names, "range"
  ensure
    turret&.free
  end

  # @behavior RD-012
  def test_an_export_takes_its_type_from_the_value_it_was_declared_with
    turret = Loader::Turret.new
    script = Godot::ResourceLoader.load("res://loader/turret.rb")

    assert_equal 300.0, script.get_property_default_value(:range)
  ensure
    turret&.free
  end

  # @behavior RD-013
  def test_a_scripts_properties_include_the_ones_the_class_it_extends_exported
    cannon = Loader::Cannon.new
    script = Godot::ResourceLoader.load("res://loader/cannon.rb")

    names = script.get_script_property_list.map { |property| property["name"] }

    assert_equal %w[barrel mode range], names
  ensure
    cannon&.free
  end

  # @behavior RD-014
  def test_an_export_declared_with_no_value_is_refused
    assert_raises(ArgumentError) { Loader::Valueless }
  end

  # @behavior RD-015
  def test_an_export_a_ruby_superclass_declared_is_refused
    assert_raises(ArgumentError) { Loader::Shadowing }
  end

  # @behavior RD-016
  def test_an_export_the_engine_class_already_has_is_refused
    assert_raises(ArgumentError) { Loader::Overriding }
  end

  # @behavior RD-017
  def test_an_export_ruby_objects_already_answer_is_refused
    assert_raises(ArgumentError) { Loader::Obscuring }
  end

  # @behavior RD-018
  def test_an_export_declared_again_as_it_stands_is_no_new_property
    restating = Loader::Restating.new
    script = Godot::ResourceLoader.load("res://loader/restating.rb")

    mode = script.get_script_property_list.select { |property| property["name"] == "mode" }

    assert_equal 1, mode.size
  ensure
    restating&.free
  end

  # @behavior RD-019
  def test_an_export_declared_again_with_another_value_is_refused
    assert_raises(ArgumentError) { Loader::Differing }
  end

  # @behavior RD-020
  def test_a_signal_a_ruby_superclass_declared_is_refused
    assert_raises(ArgumentError) { Loader::Resounding }
  end

  # @behavior RD-021
  def test_a_node_answers_an_exported_property_by_the_accessor_its_class_gets
    turret = Loader::Turret.new

    assert_equal :nearest, turret.mode
  ensure
    turret&.free
  end

  # @behavior RD-022
  def test_a_classs_own_accessor_stands_in_place_of_the_one_an_export_would_define
    reading = Loader::Reading.new

    assert_equal 600.0, reading.range
  ensure
    reading&.free
  end

  # @behavior RD-023
  def test_an_exported_value_is_written_before_the_object_is_initialized
    priming = Loader::Priming.new

    assert_equal 300.0, priming.primed
  ensure
    priming&.free
  end

  # @behavior RD-024
  def test_each_node_has_its_own_copy_of_a_container_it_was_exported
    one = Loader::Stacking.new
    other = Loader::Stacking.new

    one.rounds << 3

    assert_equal [1, 2], other.rounds
  ensure
    one&.free
    other&.free
  end

  # @behavior RD-025
  def test_a_node_lists_the_properties_its_class_exported
    turret = Loader::Turret.new

    names = turret.get_property_list.map { |property| property["name"] }

    assert_includes names, "range"
  ensure
    turret&.free
  end

  # @behavior RD-026
  def test_godot_writes_an_exported_property_through_the_accessor
    turret = Loader::Turret.new

    turret.set(:range, 500.0)

    assert_equal 500.0, turret.range
  ensure
    turret&.free
  end

  # @behavior RD-027
  def test_a_property_the_engine_class_has_stays_the_engines
    masking = Loader::Masking.new

    masking.set(:position, Godot::Vector2.new(1, 2))

    assert_equal Godot::Vector2.new(1, 2), masking.position
  ensure
    masking&.free
  end

  # @behavior RD-028
  def test_godot_reads_an_instance_variable_the_class_did_not_export
    holding = Loader::Holding.new

    assert_equal 7, holding.get(:kept)
  ensure
    holding&.free
  end

  # @behavior RD-029
  def test_godot_writes_an_instance_variable_the_class_did_not_export
    holding = Loader::Holding.new

    holding.set(:kept, 9)

    assert_equal 9, holding.kept
  ensure
    holding&.free
  end

  # @behavior RD-033
  def test_a_copy_of_a_node_carries_what_its_properties_hold
    turret = Loader::Turret.new
    turret.set(:range, 500.0)

    copy = turret.duplicate

    assert_equal 500.0, copy.get(:range)
  ensure
    turret&.free
    copy&.free
  end
end
