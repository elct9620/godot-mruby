class ExportsTest < Minitest::Test
  # @GlobalScope's PropertyUsageFlags value marking a script's own variable,
  # which the headings in a property list carry none of.
  SCRIPT_VARIABLE = 4096

  # @behavior RD-011
  def test_an_exported_property_is_one_the_script_has
    turret = autofree(Turret.new)
    script = Godot::ResourceLoader.load("res://src/turret.rb")

    names = script.get_script_property_list.map { |property| property["name"] }

    assert_includes names, "range"
  end

  # @behavior RD-012
  def test_an_export_takes_its_type_from_the_value_it_was_declared_with
    turret = autofree(Turret.new)
    script = Godot::ResourceLoader.load("res://src/turret.rb")

    assert_equal 300.0, script.get_property_default_value(:range)
  end

  # @behavior RD-013
  def test_a_scripts_properties_include_the_ones_the_class_it_extends_exported
    cannon = autofree(Unit::Declarations::Cannon.new)
    script = Godot::ResourceLoader.load("res://test/unit/declarations/cannon.rb")

    names = read_export_names(script)

    assert_equal %w[barrel mode range], names
  end

  # @behavior RD-014
  def test_an_export_declared_with_no_value_is_refused
    error = assert_raises(ArgumentError) { Unit::Declarations::Blank }

    assert_equal "res://test/unit/declarations/blank.rb:5", Minitest.filter_backtrace(error.backtrace).first
    assert_equal %(Cannot use "export" because the type of the initialized value can't be inferred.),
                 error.message
  end

  # @behavior RD-015
  def test_an_export_a_ruby_superclass_declared_is_refused
    error = assert_raises(ArgumentError) { Unit::Declarations::Decoy }

    assert_equal "res://test/unit/declarations/decoy.rb:5", Minitest.filter_backtrace(error.backtrace).first
    assert_equal 'The member "mode" already exists in parent class Turret.', error.message
  end

  # @behavior RD-016
  def test_an_export_the_engine_class_already_has_is_refused
    error = assert_raises(ArgumentError) { Unit::Declarations::Overlay }

    assert_equal "res://test/unit/declarations/overlay.rb:5", Minitest.filter_backtrace(error.backtrace).first
    assert_equal %(Member "position" redefined (original in native class 'Node2D')), error.message
  end

  # @behavior RD-017
  def test_an_export_ruby_objects_already_answer_is_refused
    error = assert_raises(ArgumentError) { Unit::Declarations::Veil }

    assert_equal "res://test/unit/declarations/veil.rb:6", Minitest.filter_backtrace(error.backtrace).first
  end

  # @behavior RD-018
  def test_an_export_declared_again_as_it_stands_is_no_new_property
    ledger = autofree(Unit::Declarations::Ledger.new)
    script = Godot::ResourceLoader.load("res://test/unit/declarations/ledger.rb")

    mode = script.get_script_property_list.select { |property| property["name"] == "mode" }

    assert_equal 1, mode.size
  end

  # @behavior RD-019
  def test_an_export_declared_again_with_another_value_is_refused
    error = assert_raises(ArgumentError) { Unit::Declarations::Lever }

    assert_equal "res://test/unit/declarations/lever.rb:6", Minitest.filter_backtrace(error.backtrace).first
  end

  # @behavior RD-020
  def test_a_signal_a_ruby_superclass_declared_is_refused
    error = assert_raises(ArgumentError) { Unit::Declarations::Resonator }

    assert_equal "res://test/unit/declarations/resonator.rb:5", Minitest.filter_backtrace(error.backtrace).first
  end

  # @behavior RD-021
  def test_a_node_answers_an_exported_property_by_the_accessor_its_class_gets
    turret = autofree(Turret.new)

    assert_equal :nearest, turret.mode
  end

  # @behavior RD-022
  def test_a_classs_own_accessor_stands_in_place_of_the_one_an_export_would_define
    reader = autofree(Unit::Declarations::Reader.new)

    assert_equal 600.0, reader.range
  end

  # @behavior RD-023
  def test_an_exported_value_is_written_before_the_object_is_initialized
    primer = autofree(Unit::Declarations::Primer.new)

    assert_equal 300.0, primer.initial_range
  end

  # @behavior RD-024
  def test_each_node_has_its_own_copy_of_a_container_it_was_exported
    one = autofree(Unit::Declarations::Crate.new)
    other = autofree(Unit::Declarations::Crate.new)

    one.rounds << 3

    assert_operator other.rounds, :==, [1, 2]
  end

  # @behavior RD-081
  def test_an_exported_container_is_the_one_gdscript_reaches
    crate = autofree(Unit::Declarations::Crate.new)

    appender.append_to(crate)

    assert_operator crate.rounds, :==, [1, 2, 3]
  end

  # @behavior RD-082
  def test_an_array_ruby_writes_to_an_exported_property_is_held_as_the_engines_container
    crate = autofree(Unit::Declarations::Crate.new)
    written = [7]
    crate.rounds = written

    appender.append_to(crate)

    assert_operator crate.rounds, :==, [7, 3]
    assert_equal [7], written
  end

  # @behavior RD-025
  def test_a_node_lists_the_properties_its_class_exported
    turret = autofree(Turret.new)

    names = turret.get_property_list.map { |property| property["name"] }

    assert_includes names, "range"
  end

  # @behavior RD-026
  def test_godot_writes_an_exported_property_through_the_accessor
    turret = autofree(Turret.new)

    turret.set(:range, 500.0)

    assert_equal 500.0, turret.range
  end

  # @behavior RD-027
  def test_a_property_the_engine_class_has_stays_the_engines
    mask = autofree(Unit::Declarations::Mask.new)

    mask.set(:position, Godot::Vector2.new(1, 2))

    assert_equal Godot::Vector2.new(1, 2), mask.position
  end

  # @behavior RD-028
  def test_godot_reads_an_instance_variable_the_class_did_not_export
    pouch = autofree(Unit::Declarations::Pouch.new)

    assert_equal 7, pouch.get(:stash)
  end

  # @behavior RD-029
  def test_godot_writes_an_instance_variable_the_class_did_not_export
    pouch = autofree(Unit::Declarations::Pouch.new)

    pouch.set(:stash, 9)

    assert_equal 9, pouch.stash
  end

  # @behavior RD-033
  def test_a_copy_of_a_node_carries_what_its_properties_hold
    turret = autofree(Turret.new)
    turret.set(:range, 500.0)

    copy = autofree(turret.duplicate)

    assert_equal 500.0, copy.get(:range)
  end

  # The names of the properties a script has, without the headings they are
  # listed under.
  def read_export_names(script)
    script.get_script_property_list
          .select { |member| member["usage"] & SCRIPT_VARIABLE != 0 }
          .map { |property| property["name"] }
  end

  # @behavior RD-034
  def test_a_property_only_the_nodes_own_class_has_stays_the_engines
    sprite = autofree(covered_sprite)

    sprite.set(:centered, false)

    refute sprite.is_centered
  end

  # @behavior RD-035
  def test_a_variable_named_as_a_property_of_the_nodes_own_class_is_untouched
    sprite = autofree(covered_sprite)

    sprite.set(:centered, false)

    assert_equal "mine", sprite.call(:cover)
  end

  # @behavior RD-077
  def test_a_property_declared_with_a_resource_a_file_holds_answers_it_as_its_default
    script = Godot::ResourceLoader.load("res://test/unit/declarations/prism.rb")
    Unit::Declarations::Prism

    sheen = script.get_property_default_value(:sheen)

    assert_equal Godot::ResourceLoader.load("res://test/unit/declarations/sheen.tres"), sheen
  end

  # @behavior RD-078
  def test_a_property_declared_with_an_object_no_file_holds_has_no_default
    script = Godot::ResourceLoader.load("res://test/unit/declarations/prism.rb")
    Unit::Declarations::Prism

    assert_nil script.get_property_default_value(:tint)
  end

  # @behavior RD-079
  def test_a_container_declared_with_resources_a_file_holds_answers_them_in_its_default
    script = Godot::ResourceLoader.load("res://test/unit/declarations/prism.rb")
    sheen = Unit::Declarations::Prism::SHEEN

    palette = script.get_property_default_value(:palette)

    assert_operator palette, :==, [sheen, { "glow" => sheen }]
  end

  # @behavior RD-080
  def test_a_container_holding_an_object_no_file_holds_has_no_default
    script = Godot::ResourceLoader.load("res://test/unit/declarations/prism.rb")
    Unit::Declarations::Prism

    assert_nil script.get_property_default_value(:blend)
  end

  private

  # A Sprite2D whose Ruby object, of a class extending Node2D, is built and
  # has written its own @centered.
  def covered_sprite
    sprite = Godot::Sprite2D.new
    sprite.set_script(Godot::ResourceLoader.load("res://test/unit/declarations/cloak.rb"))
    sprite.call(:cover)
    sprite
  end

  # A GDScript object whose `append_to` appends 3 to a node's `rounds`, as
  # GDScript code reaching the node's property does.
  def appender
    script = Godot::GDScript.new
    script.source_code = "extends RefCounted\nfunc append_to(node):\n\tnode.rounds.append(3)\n"
    script.reload
    script.new
  end
end
