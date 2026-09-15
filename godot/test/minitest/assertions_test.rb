# Each assertion passes on what it asserts and raises Minitest::Assertion on
# anything else.
class AssertionsTest < Minitest::Test
  # @behavior RM-004
  def test_an_assertion_that_holds_raises_nothing
    assert true
    refute nil
    refute_equal 1, 2
    assert_nil nil
    refute_nil false
    assert_empty []
    refute_empty [1]
    assert_in_delta 1.0, 1.05, 0.1
    refute_in_delta 1.0, 1.2, 0.1
    assert_in_epsilon 100, 101, 0.02
    refute_in_epsilon 100, 110, 0.02
    assert_includes %w[potion ether], "potion"
    refute_includes %w[potion ether], "elixir"
    assert_instance_of Integer, 1
    refute_instance_of Numeric, 1
    assert_kind_of Numeric, 1
    refute_kind_of String, 1
    assert_operator 2, :>, 1
    refute_operator 1, :>, 2
    assert_operator [], :empty?
    assert_predicate nil, :nil?
    refute_predicate 1, :nil?
    assert_respond_to "potion", :upcase
    refute_respond_to "potion", :fly
    item = "potion"
    assert_same item, item
    refute_same "potion", "potion"
  end

  # @behavior RM-005
  def test_pass_counts_an_assertion
    before = assertions
    pass
    assert_equal before + 1, assertions
  end

  # @behavior RM-006
  def test_a_given_message_comes_before_the_assertions_own
    error = assert_raises(Minitest::Assertion) { assert_includes [1, 2], 3, "the inventory" }
    assert_equal "the inventory.\nExpected [1, 2] to include 3.", error.message
  end

  # @behavior RM-007
  def test_a_failed_assert_equal_shows_the_expected_value_before_the_actual_one
    error = assert_raises(Minitest::Assertion) { assert_equal 1, 2 }
    assert_equal "Expected: 1\n  Actual: 2", error.message
  end

  # @behavior RM-008
  def test_assert_equal_refuses_nil_as_the_expected_value
    error = assert_raises(Minitest::Assertion) { assert_equal nil, nil }
    assert_equal "Use assert_nil if expecting nil.", error.message
  end

  # @behavior RM-009
  def test_assert_fails_on_a_falsy_value
    assert_raises(Minitest::Assertion) { assert false }
  end

  # @behavior RM-010
  def test_refute_fails_on_a_truthy_value
    assert_raises(Minitest::Assertion) { refute 1 }
  end

  # @behavior RM-011
  def test_refute_equal_fails_on_equal_values
    assert_raises(Minitest::Assertion) { refute_equal 1, 1 }
  end

  # @behavior RM-012
  def test_assert_nil_fails_on_a_value_other_than_nil
    assert_raises(Minitest::Assertion) { assert_nil false }
  end

  # @behavior RM-013
  def test_refute_nil_fails_on_nil
    assert_raises(Minitest::Assertion) { refute_nil nil }
  end

  # @behavior RM-014
  def test_assert_raises_answers_the_exception_the_block_raised
    error = assert_raises(ArgumentError) { raise ArgumentError, "wrong" }
    assert_equal "wrong", error.message
  end

  # @behavior RM-015
  def test_assert_raises_fails_when_the_block_raises_nothing
    assert_raises(Minitest::Assertion) { assert_raises(ArgumentError) { :nothing } }
  end

  # @behavior RM-016
  def test_assert_raises_fails_when_the_block_raises_another_exception
    assert_raises(Minitest::Assertion) { assert_raises(ArgumentError) { raise TypeError } }
  end

  # @behavior RM-017
  def test_flunk_fails
    assert_raises(Minitest::Assertion) { flunk }
  end

  # @behavior RM-018
  def test_skip_ends_the_test_with_the_message_it_is_given
    skipped = assert_raises(Minitest::Skip) { skip "later" }
    assert_equal "later", skipped.message
  end

  # @behavior RM-019
  def test_assert_empty_fails_on_a_collection_holding_something
    assert_raises(Minitest::Assertion) { assert_empty [1] }
  end

  # @behavior RM-020
  def test_refute_empty_fails_on_an_empty_collection
    assert_raises(Minitest::Assertion) { refute_empty "" }
  end

  # @behavior RM-021
  def test_assert_in_delta_fails_beyond_the_delta_saying_how_far_apart
    error = assert_raises(Minitest::Assertion) { assert_in_delta 1, 3, 1 }
    assert_equal "Expected |1 - 3| (2) to be <= 1.", error.message
  end

  # @behavior RM-022
  def test_refute_in_epsilon_fails_within_epsilon_of_the_smaller_magnitude
    assert_raises(Minitest::Assertion) { refute_in_epsilon 100, 101, 0.02 }
  end

  # @behavior RM-023
  def test_refute_includes_fails_on_a_member
    assert_raises(Minitest::Assertion) { refute_includes "potion", "ion" }
  end

  # @behavior RM-024
  def test_assert_instance_of_fails_on_another_class_naming_both
    error = assert_raises(Minitest::Assertion) { assert_instance_of String, 1 }
    assert_equal "Expected 1 to be an instance of String, not Integer.", error.message
  end

  # @behavior RM-025
  def test_assert_kind_of_fails_on_an_object_of_another_kind
    assert_raises(Minitest::Assertion) { assert_kind_of String, 1 }
  end

  # @behavior RM-026
  def test_assert_operator_fails_when_the_operator_answers_false_naming_it
    error = assert_raises(Minitest::Assertion) { assert_operator 1, :>, 2 }
    assert_equal "Expected 1 to be > 2.", error.message
  end

  # @behavior RM-027
  def test_assert_predicate_fails_when_the_predicate_answers_false
    assert_raises(Minitest::Assertion) { assert_predicate 1, :nil? }
  end

  # @behavior RM-028
  def test_assert_respond_to_fails_naming_the_objects_class
    error = assert_raises(Minitest::Assertion) { assert_respond_to 1, :fly }
    assert_equal "Expected 1 (Integer) to respond to #fly.", error.message
  end

  # @behavior RM-029
  def test_assert_same_fails_on_distinct_objects_naming_their_ids
    error = assert_raises(Minitest::Assertion) { assert_same :a, :b }
    assert_equal "Expected :b (oid=#{:b.object_id}) to be the same as :a (oid=#{:a.object_id}).", error.message
  end
end
