# Tests named in order, each printing its name as it runs, so a run shows the
# order its seed gave the classes and their tests.
class AlphaOrderTest < Minitest::Test
  %w[test_a test_b test_c].each do |name|
    define_method(name) { puts "ran #{self.class}##{name}" }
  end
end

class BetaOrderTest < Minitest::Test
  %w[test_d test_e test_f].each do |name|
    define_method(name) { puts "ran #{self.class}##{name}" }
  end
end
