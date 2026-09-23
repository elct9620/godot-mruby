# A mock answers only the calls it expects, and a stub replaces a method only
# within its block.
class MockTest < Minitest::Test
  def setup
    @inventory = Minitest::Mock.new
  end

  # @behavior RM-030
  def test_a_mock_answers_an_expected_call_with_its_return_value
    @inventory.expect(:count, 3, [:potion])

    assert_equal 3, @inventory.count(:potion)
    assert_mock @inventory
  end

  # @behavior RM-031
  def test_a_mock_matches_an_argument_by_its_class
    @inventory.expect(:add, true, [Symbol])

    assert @inventory.add(:ether)
  end

  # @behavior RM-032
  def test_a_mock_answers_repeated_expectations_in_order
    @inventory.expect(:take, :potion)
    @inventory.expect(:take, :ether)

    assert_equal %i[potion ether], [@inventory.take, @inventory.take]
  end

  # @behavior RM-033
  def test_a_mock_checks_its_arguments_with_a_block
    @inventory.expect(:add, true) { |item, amount| item == :potion && amount > 0 }

    assert @inventory.add(:potion, 2)
  end

  # @behavior RM-034
  def test_a_mock_refuses_a_call_with_arguments_it_did_not_expect
    @inventory.expect(:count, 3, [:potion])

    assert_raises(MockExpectationError) { @inventory.count(:ether) }
  end

  # @behavior RM-035
  def test_a_mock_refuses_a_call_it_did_not_expect
    error = assert_raises(NoMethodError) { @inventory.drop(:potion) }
    assert_equal "unmocked method :drop, expected one of []", error.message
  end

  # @behavior RM-036
  def test_assert_mock_fails_where_the_test_asserts_for_a_call_never_made
    @inventory.expect(:count, 3, [:potion])

    error = assert_raises(Minitest::Assertion) { assert_mock @inventory }

    assert_equal "Expected count(:potion) => 3.", error.message
    assert_equal "res://test/unit/minitest/mock_test.rb:#{__LINE__ - 3}", error.location
  end

  # @behavior RM-037
  def test_a_stub_replaces_a_method_only_within_its_block
    item = "potion"

    inside = item.stub(:upcase, "STUBBED") { item.upcase }

    assert_equal "STUBBED", inside
    assert_equal "POTION", item.upcase
  end

  # @behavior RM-038
  def test_a_callable_stub_is_called_with_the_arguments
    item = "potion"

    item.stub(:include?, ->(part) { "looked for #{part}" }) do
      assert_equal "looked for ion", item.include?("ion")
    end
  end
end
