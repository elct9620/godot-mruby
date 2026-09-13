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
end
