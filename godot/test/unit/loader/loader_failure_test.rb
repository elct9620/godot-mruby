class LoaderFailureTest < Minitest::Test
  # @behavior RL-015
  def test_a_constant_needed_while_its_own_file_runs_raises_name_error_naming_the_cycle
    error = assert_raises(NameError) { Unit::Loader::CycleA }
    assert_includes error.message, "res://test/unit/loader/cycle_a.rb -> res://test/unit/loader/cycle_b.rb -> res://test/unit/loader/cycle_a.rb"
  end

  # @behavior RL-016
  def test_a_file_that_runs_without_defining_its_constant_raises_name_error
    error = assert_raises(NameError) { Unit::Loader::Ghost }
    assert_equal "res://test/unit/loader/ghost.rb ran without defining Unit::Loader::Ghost", error.message
  end

  # @behavior RL-017
  def test_a_file_that_raises_takes_the_constants_it_created_with_it
    assert_raises(RuntimeError) { Unit::Loader::Tripwire }
    refute Unit::Loader.const_defined?(:Tripwire)
    refute Unit::Loader.const_defined?(:TripwireError)
  end

  # @behavior RL-018
  def test_a_file_loaded_while_a_failing_file_ran_keeps_its_constants
    assert_raises(RuntimeError) { Unit::Loader::Tripwire }
    assert Unit::Loader.const_defined?(:Keepsake)
  end

  # @behavior RL-022
  def test_a_directory_module_made_while_a_failing_file_ran_stays
    assert_raises(RuntimeError) { Unit::Loader::Tripwire }
    assert Unit::Loader.const_defined?(:Shelf)
  end

  # @behavior RL-044
  def test_a_file_whose_own_class_needs_a_file_that_raises_takes_its_class_with_it
    assert_raises(RuntimeError) { Unit::Loader::Chest }
    refute Unit::Loader.const_defined?(:Chest)
  end

  # @behavior RL-023
  def test_the_hook_that_tells_the_loader_of_new_constants_stays_private
    assert_raises(NoMethodError) { Module.const_added(:Unused) }
  end

  # @behavior RL-019
  def test_a_file_that_raised_runs_again_at_the_next_use_of_its_constant
    runs = -> { Unit::Loader.instance_variable_get(:@tripwire_runs).to_i }
    first = assert_raises(RuntimeError) { Unit::Loader::Tripwire }
    ran = runs.call
    second = assert_raises(RuntimeError) { Unit::Loader::Tripwire }
    assert_equal ran + 1, runs.call
    assert_equal first.message, second.message
  end

  # @behavior RL-020
  def test_a_file_loaded_by_name_that_does_not_parse_raises_syntax_error_at_its_line
    error = assert_raises(SyntaxError) { Unit::Loader::Wreck }
    assert_includes error.message, "res://test/unit/loader/wreck.rb:7: syntax error"
  end

  # @behavior RL-027
  def test_opening_another_files_class_with_another_superclass_raises_type_error
    error = assert_raises(TypeError) { Unit::Loader::Impostor }
    assert_includes error.message, "superclass mismatch"
  end

  # @behavior RL-028
  def test_a_directory_module_a_file_opens_is_made_before_the_file_runs
    assert_raises(RuntimeError) { Unit::Loader::Awning }
    assert Unit::Loader.const_defined?(:Shades)
  end

  # @behavior RL-029
  def test_a_node_script_extending_another_class_than_its_ancestry_says_raises_type_error
    error = assert_raises(TypeError) { Unit::Loader::Posts::Lookalike }
    assert_equal "Unit::Loader::Posts::Lookalike in res://test/unit/loader/posts/lookalike.rb extends Unit::Loader::Posts::Sentry, " \
                 "not the class res://test/unit/loader/sentry.rb names", error.message
  end

  # @behavior RL-036
  def test_the_class_index_names_no_file_under_the_template_directory
    assert_raises(NameError) { ScriptTemplates::Node::Greeting }
  end
end
