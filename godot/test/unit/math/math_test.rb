class MathTest < Minitest::Test
  # @behavior RC-001
  def test_ruby_has_math
    assert_in_delta 0.8414709848078965, Math.sin(1.0)
    assert_in_delta 1.4142135623730951, Math.sqrt(2.0)
  end

  # @behavior RC-002
  def test_a_value_outside_a_functions_domain_raises_math_domain_error
    assert_raises(Math::DomainError) { Math.sqrt(-1) }
  end

  # @behavior RC-008
  def test_rubys_numbers_answer_rubys_numeric_predicates
    assert 0.0.zero?
    assert 2.positive?
    assert(-0.5.negative?)
    assert 4.even?
  end
end
