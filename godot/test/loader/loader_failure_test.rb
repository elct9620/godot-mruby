class LoaderFailureTest < Minitest::Test
  # @behavior RL-015
  def test_a_constant_needed_while_its_own_file_runs_raises_name_error_naming_the_cycle
    error = assert_raises(NameError) { Loader::CycleA }
    assert_includes error.message, "res://loader/cycle_a.rb -> res://loader/cycle_b.rb -> res://loader/cycle_a.rb"
  end

  # @behavior RL-016
  def test_a_file_that_runs_without_defining_its_constant_raises_name_error
    error = assert_raises(NameError) { Loader::Undefined }
    assert_equal "res://loader/undefined.rb ran without defining Loader::Undefined", error.message
  end

  # @behavior RL-017
  def test_a_file_that_raises_takes_the_constants_it_created_with_it
    assert_raises(RuntimeError) { Loader::Raising }
    refute Loader.const_defined?(:Raising)
    refute Loader.const_defined?(:RaisingError)
  end

  # @behavior RL-018
  def test_a_file_loaded_while_a_failing_file_ran_keeps_its_constants
    assert_raises(RuntimeError) { Loader::Raising }
    assert Loader.const_defined?(:Kept)
  end

  # @behavior RL-022
  def test_a_directory_module_made_while_a_failing_file_ran_stays
    assert_raises(RuntimeError) { Loader::Raising }
    assert Loader.const_defined?(:Shelf)
  end

  # @behavior RL-023
  def test_the_hook_that_tells_the_loader_of_new_constants_stays_private
    assert_raises(NoMethodError) { Module.const_added(:Unused) }
  end

  # @behavior RL-019
  def test_a_file_that_raised_runs_again_at_the_next_use_of_its_constant
    first = assert_raises(RuntimeError) { Loader::Raising }
    second = assert_raises(RuntimeError) { Loader::Raising }
    assert_equal first.message, second.message
  end

  # @behavior RL-020
  def test_a_file_loaded_by_name_that_does_not_parse_raises_syntax_error_at_its_line
    error = assert_raises(SyntaxError) { Loader::Broken }
    assert_includes error.message, "res://loader/broken.rb:4: syntax error"
  end

  # @behavior RL-027
  def test_opening_another_files_class_with_another_superclass_raises_type_error
    error = assert_raises(TypeError) { Loader::Mismatching }
    assert_includes error.message, "superclass mismatch"
  end

  # @behavior RL-028
  def test_a_directory_module_a_file_opens_is_made_before_the_file_runs
    assert_raises(RuntimeError) { Loader::Shading }
    assert Loader.const_defined?(:Shades)
  end
end
