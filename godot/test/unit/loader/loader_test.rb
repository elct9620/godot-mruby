class LoaderTest < Minitest::Test
  # @behavior RL-004
  def test_a_constant_a_file_names_loads_at_its_first_use
    assert_equal 1, Unit::Loader::Inventory.new(2).add(:potion).size
  end

  # @behavior RL-005
  def test_a_namespace_file_runs_before_a_file_inside_it
    assert_equal 10, Unit::Loader::Items::Potion.new.price
  end

  # @behavior RL-006
  def test_a_directory_without_a_file_of_its_own_is_an_empty_module
    assert_instance_of Module, Unit::Loader::Ui
    assert_equal "Unit::Loader::Ui::Hud", Unit::Loader::Ui::Hud.to_s
  end

  # @behavior RL-007
  def test_a_directory_module_answers_only_to_the_name_zeitwerk_gives_it
    error = assert_raises(NameError) { Unit::Loader::UI }
    assert_equal "res://test/unit/loader/ui/ is the namespace Unit::Loader::Ui, not Unit::Loader::UI", error.message
  end

  # @behavior RL-008
  def test_a_name_inside_a_namespace_finds_that_namespaces_file
    assert_equal "Unit::Loader::Items::Potion: 10", Unit::Loader::Items::Potion.new.label
  end

  # @behavior RL-009
  def test_a_name_no_file_spells_raises_the_name_error_core_ruby_raises
    error = assert_raises(NameError) { NoSuchThing }
    assert_equal :NoSuchThing, error.name
    assert_equal "uninitialized constant LoaderTest::NoSuchThing", error.message
  end

  # @behavior RL-010
  def test_a_qualified_name_its_namespace_lacks_is_looked_for_outward
    assert_same Unit::Loader::Inventory, Unit::Loader::Items::Inventory
  end

  # @behavior RL-011
  def test_a_node_scripts_namespace_file_runs_before_it
    greeter = Godot::Node.new
    greeter.set_script(Godot::ResourceLoader.load("res://test/unit/loader/namespaced/greeter.rb"))

    assert_equal "namespace from namespaced.rb", greeter.call(:greeting)
  ensure
    greeter&.free
  end

  # @behavior RL-012
  def test_a_file_under_a_test_directory_loads_by_name
    assert_equal 0, Unit::Loader::Support::FakeClock.new.now
  end

  # @behavior RL-031
  def test_a_file_under_a_root_directory_names_its_constant_from_the_top_level
    assert_kind_of Class, Turret
    refute Object.const_defined?(:Src)
  end

  # @behavior RL-026
  def test_a_file_that_opens_another_files_class_runs_after_that_file
    Unit::Loader::Opener
    lamp = Unit::Loader::Lamp.new
    assert lamp.bright?
    assert lamp.lit?
  end

  # @behavior RL-030
  def test_a_library_files_superclass_is_rubys_to_look_up
    superclass = Unit::Loader::Posts::Plain.superclass
    assert_same Unit::Loader::Posts::Marker, superclass
  end
end
