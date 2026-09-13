class LoadingTest < Minitest::Test
  # @behavior RL-004
  def test_a_constant_a_file_names_loads_at_its_first_use
    assert_equal 1, Inventory.new(2).add(:potion).size
  end

  # @behavior RL-005
  def test_a_namespace_file_runs_before_a_file_inside_it
    assert_equal 10, Items::Potion.new.price
  end

  # @behavior RL-006
  def test_a_directory_without_a_file_of_its_own_is_an_empty_module
    assert_instance_of Module, Ui
    assert_equal "Ui::Hud", Ui::Hud.to_s
  end

  # @behavior RL-007
  def test_a_directory_module_answers_only_to_the_name_zeitwerk_gives_it
    error = assert_raises(NameError) { UI }
    assert_equal "res://ui/ is the namespace Ui, not UI", error.message
  end

  # @behavior RL-008
  def test_a_name_inside_a_namespace_finds_that_namespaces_file
    assert_equal "Items::Potion: 10", Items::Potion.new.label
  end

  # @behavior RL-009
  def test_a_name_no_file_spells_raises_the_name_error_core_ruby_raises
    error = assert_raises(NameError) { NoSuchThing }
    assert_equal :NoSuchThing, error.name
    assert_equal "uninitialized constant LoadingTest::NoSuchThing", error.message
  end

  # @behavior RL-010
  def test_a_qualified_name_its_namespace_lacks_is_looked_for_outward
    assert_same Inventory, Items::Inventory
  end

  # @behavior RL-012
  def test_test_support_loads_by_name
    assert_equal 0, Test::Support::FakeClock.new.now
  end

  # @behavior RL-013
  def test_a_test_class_loaded_by_name_during_the_run_does_not_run
    assert_includes Minitest::Runnable.runnables, Test::Support::LateTestCase
  end
end
