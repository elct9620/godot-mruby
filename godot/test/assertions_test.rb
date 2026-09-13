# Each assertion passes on what it asserts and raises Minitest::Assertion on
# anything else.
class AssertionsTest < Minitest::Test
  # @behavior RT-011
  def test_a_failed_assert_equal_shows_the_expected_value_before_the_actual_one
    error = assert_raises(Minitest::Assertion) { assert_equal 1, 2 }
    assert_equal "Expected: 1\n  Actual: 2", error.message
  end

  def test_assert_and_refute_judge_truthiness
    assert true
    refute nil
    assert_raises(Minitest::Assertion) { assert false }
    assert_raises(Minitest::Assertion) { refute 1 }
  end

  def test_refute_equal_fails_on_equal_values
    refute_equal 1, 2
    assert_raises(Minitest::Assertion) { refute_equal 1, 1 }
  end

  def test_assert_nil_and_refute_nil_judge_nil
    assert_nil nil
    refute_nil false
    assert_raises(Minitest::Assertion) { assert_nil false }
    assert_raises(Minitest::Assertion) { refute_nil nil }
  end

  def test_assert_raises_returns_what_the_block_raised
    error = assert_raises(ArgumentError) { raise ArgumentError, "wrong" }
    assert_equal "wrong", error.message
  end

  def test_assert_raises_fails_when_nothing_or_something_else_is_raised
    assert_raises(Minitest::Assertion) { assert_raises(ArgumentError) { :nothing } }
    assert_raises(Minitest::Assertion) { assert_raises(ArgumentError) { raise TypeError } }
  end

  def test_flunk_fails_and_skip_ends_without_failing
    assert_raises(Minitest::Assertion) { flunk }
    skipped = assert_raises(Minitest::Skip) { skip "later" }
    assert_equal "later", skipped.message
  end

  # @behavior RT-014
  def test_a_given_message_comes_before_the_assertions_own
    error = assert_raises(Minitest::Assertion) { assert_includes [1, 2], 3, "the inventory" }
    assert_equal "the inventory.\nExpected [1, 2] to include 3.", error.message
  end

  # @behavior RT-015
  def test_assert_equal_refuses_nil_as_the_expected_value
    error = assert_raises(Minitest::Assertion) { assert_equal nil, nil }
    assert_equal "Use assert_nil if expecting nil.", error.message
  end

  def test_pass_counts_an_assertion
    before = assertions
    pass
    assert_equal before + 1, assertions
  end

  def test_assert_empty_and_refute_empty_judge_emptiness
    assert_empty []
    refute_empty [1]
    assert_raises(Minitest::Assertion) { assert_empty [1] }
    assert_raises(Minitest::Assertion) { refute_empty "" }
  end

  def test_in_delta_and_in_epsilon_judge_closeness
    assert_in_delta 1.0, 1.05, 0.1
    refute_in_delta 1.0, 1.2, 0.1
    assert_in_epsilon 100, 101, 0.02
    refute_in_epsilon 100, 110, 0.02
    error = assert_raises(Minitest::Assertion) { assert_in_delta 1, 3, 1 }
    assert_equal "Expected |1 - 3| (2) to be <= 1.", error.message
    assert_raises(Minitest::Assertion) { refute_in_epsilon 100, 101, 0.02 }
  end

  def test_includes_judges_membership
    assert_includes %w[potion ether], "potion"
    refute_includes %w[potion ether], "elixir"
    assert_raises(Minitest::Assertion) { refute_includes "potion", "ion" }
  end

  def test_instance_of_and_kind_of_judge_the_class
    assert_instance_of Integer, 1
    refute_instance_of Numeric, 1
    assert_kind_of Numeric, 1
    refute_kind_of String, 1
    error = assert_raises(Minitest::Assertion) { assert_instance_of String, 1 }
    assert_equal "Expected 1 to be an instance of String, not Integer.", error.message
    assert_raises(Minitest::Assertion) { assert_kind_of String, 1 }
  end

  def test_operator_and_predicate_send_the_operator
    assert_operator 2, :>, 1
    refute_operator 1, :>, 2
    assert_operator [], :empty?
    assert_predicate nil, :nil?
    refute_predicate 1, :nil?
    error = assert_raises(Minitest::Assertion) { assert_operator 1, :>, 2 }
    assert_equal "Expected 1 to be > 2.", error.message
    assert_raises(Minitest::Assertion) { assert_predicate 1, :nil? }
  end

  def test_respond_to_judges_the_methods_an_object_answers
    assert_respond_to "potion", :upcase
    refute_respond_to "potion", :fly
    error = assert_raises(Minitest::Assertion) { assert_respond_to 1, :fly }
    assert_equal "Expected 1 (Integer) to respond to #fly.", error.message
  end

  def test_same_judges_identity_rather_than_equality
    item = "potion"
    assert_same item, item
    refute_same "potion", "potion"
    error = assert_raises(Minitest::Assertion) { assert_same :a, :b }
    assert_equal "Expected :b (oid=#{:b.object_id}) to be the same as :a (oid=#{:a.object_id}).", error.message
  end
end
