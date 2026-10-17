class CoreLibraryTest < Minitest::Test
  # @behavior RJ-001
  def test_a_string_answers_rubys_string_methods
    assert "res://player.rb".start_with?("res://")
    assert "player.rb".end_with?(".rb")
    assert_equal "jump", "  jump\n".strip
    assert_equal %w[h p], "hp".chars
  end

  # @behavior RJ-002
  def test_an_array_answers_rubys_array_methods
    assert_equal [1, 2], [1, 1, 2].uniq
    assert_equal [1, 2], [1, nil, 2].compact
    assert_equal [1, 2, 3], [1, [2, [3]]].flatten
    assert_equal 3, [1, [2, 3]].dig(1, 1)
    assert_equal :none, [].fetch(0, :none)
  end

  # @behavior RJ-003
  def test_a_collection_answers_rubys_enumerable_methods
    hit_points = { slime: 3, bat: 1, golem: 8 }

    assert_equal 12, hit_points.values.sum
    assert_equal :bat, hit_points.min_by { |_, hp| hp }.first
    assert_equal %i[bat slime golem], hit_points.sort_by { |_, hp| hp }.map(&:first)
    assert_equal({ true => [3, 1], false => [8] }, hit_points.values.group_by(&:odd?))
    assert_equal [[3, 1], [8]], hit_points.values.each_slice(2).to_a
    assert_equal 2, hit_points.values.count(&:odd?)
  end

  # @behavior RJ-004
  def test_an_iterator_without_a_block_answers_an_enumerator
    assert_equal %w[0:a 1:b], %w[a b].each_with_index.map { |letter, index| "#{index}:#{letter}" }
    assert_equal [0, 2, 4], 3.times.map { |n| n * 2 }
  end

  # @behavior RJ-005
  def test_a_hash_answers_rubys_hash_methods
    stats = { hp: 3, gear: { sword: 2 } }

    assert_equal 3, stats.fetch(:hp)
    assert_equal 0, stats.fetch(:mp, 0)
    assert_equal 2, stats.dig(:gear, :sword)
    assert_equal({ sword: 4 }, stats[:gear].transform_values { |level| level * 2 })
  end

  # @behavior RJ-006
  def test_a_comparable_value_answers_clamp
    assert_equal 10, 15.clamp(0, 10)
    assert_in_delta 0.5, 0.5.clamp(0.0, 1.0)
  end

  # @behavior RJ-007
  def test_a_range_answers_rubys_range_methods
    assert_equal [1, 2], (1..10).first(2)
    assert_equal [9, 10], (1..10).last(2)
    assert_equal 10, (1..10).size
  end

  # @behavior RJ-008
  def test_kernel_converts_text_as_ruby_does
    assert_equal 12, Integer("12")
    assert_in_delta 1.5, Float("1.5")
    assert_raises(ArgumentError) { Integer("twelve") }
  end

  # @behavior RJ-009
  def test_struct_makes_a_class_of_named_members
    point = Struct.new(:x, :y).new(1, 2)

    assert_equal [1, 2], [point.x, point.y]
  end

  # @behavior RJ-010
  def test_data_makes_a_class_of_values_never_changed
    hit = Data.define(:damage, :critical).new(damage: 3, critical: false)

    assert_equal 3, hit.damage
    refute_respond_to hit, :damage=
    assert_equal({ damage: 3, critical: false }, hit.to_h)
  end

  # @behavior RJ-011
  def test_set_holds_each_element_once
    seen = Set.new(%i[slime bat slime])

    assert_equal 2, seen.size
    assert_includes seen, :bat
  end
end
