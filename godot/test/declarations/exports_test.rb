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
end
