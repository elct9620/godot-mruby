class RerunTest < Minitest::Test
  DIRECTORY = "res://test/unit/reload".freeze

  COUNTER = <<~RUBY.freeze
    module Unit
      module Reload
        class Counter < Godot::Node
          def bump
            @count = (@count || 0) + 1
          end

          def answer
            "after"
          end

          def added
            "added"
          end
        end
      end
    end
  RUBY

  UNFINISHED = <<~RUBY.freeze
    module Unit
      module Reload
        class Counter < Godot::Node
          LIMIT = 3
          raise "Counter is not finished"
        end
      end
    end
  RUBY

  RUBBLE = <<~RUBY.freeze
    module Unit
      module Reload
        class Rubble < Godot::Node
          def answer
            "mended"
          end
        end
      end
    end
  RUBY

  SNAG = <<~RUBY.freeze
    module Unit
      module Reload
        class Snag < Godot::Node
          def answer
            "mended"
          end
        end
      end
    end
  RUBY

  EXPORTER = <<~RUBY.freeze
    module Unit
      module Reload
        class Exporter < Godot::Node
          def speed?
            instance_variable_defined?(:@speed)
          end
        end
      end
    end
  RUBY

  ARMOR = <<~RUBY.freeze
    module Unit
      module Reload
        class Armor < Godot::Node
          export :armor, 5

          def build; end

          def wear
            @armor = 9
          end
        end
      end
    end
  RUBY

  FASTER = <<~RUBY.freeze
    module Unit
      module Reload
        class Exporter < Godot::Node
          export :speed, 20

          def speed?
            instance_variable_defined?(:@speed)
          end
        end
      end
    end
  RUBY

  # @behavior RF-002
  def test_a_reloaded_node_scripts_object_answers_a_method_only_its_changed_source_defines
    node = with_node(:counter)
    node.call(:answer)

    stub_source(:counter, COUNTER) { assert_equal "added", node.call(:added) }
  end

  # @behavior RF-003
  def test_a_reloaded_node_scripts_object_keeps_its_state
    node = with_node(:counter)
    node.call(:bump)

    stub_source(:counter, COUNTER) { assert_equal 2, node.call(:bump) }
  end

  # @behavior RF-004
  def test_a_method_the_changed_source_no_longer_defines_stays_until_the_game_restarts
    node = with_node(:counter)
    node.call(:answer)

    stub_source(:counter, COUNTER) { assert_equal "dropped", node.call(:dropped) }
  end

  # @behavior RF-005
  def test_a_changed_source_that_raises_leaves_the_class_as_it_was
    node = with_node(:counter)
    node.call(:answer)

    stub_source(:counter, UNFINISHED) { assert_equal "before", node.call(:answer) }
  end

  # @behavior RF-009
  def test_a_changed_source_that_raises_keeps_the_constants_it_assigned_again
    with_node(:counter).call(:answer)

    stub_source(:counter, UNFINISHED) { assert Unit::Reload::Counter.const_defined?(:LIMIT, false) }
  end

  # @behavior RF-006
  def test_a_node_script_whose_file_raised_makes_objects_again_once_its_changed_source_runs
    with_node(:rubble).call(:answer)

    stub_source(:rubble, RUBBLE) { assert_equal "mended", with_node(:rubble).call(:answer) }
  end

  # @behavior RF-007
  def test_a_node_whose_object_failed_stays_without_one_after_its_file_reloads
    node = with_node(:snag)
    node.call(:answer)

    stub_source(:snag, SNAG) { assert_nil node.call(:answer) }
  end

  # @behavior RF-008
  def test_a_property_the_changed_source_no_longer_exports_is_not_given_to_a_new_object
    with_node(:exporter).call(:speed?)

    stub_source(:exporter, EXPORTER) { refute with_node(:exporter).call(:speed?) }
  end

  # @behavior RF-010
  def test_a_property_the_changed_source_exports_anew_is_given_to_an_object_made_before
    node = with_node(:armor)
    node.call(:build)

    stub_source(:armor, ARMOR) { assert_equal 5, node.get(:armor) }
  end

  # @behavior RF-011
  def test_an_object_keeps_the_value_it_holds_for_a_property_the_changed_source_exports_anew
    node = with_node(:armor)
    node.call(:wear)

    stub_source(:armor, ARMOR) { assert_equal 9, node.get(:armor) }
  end

  # @behavior RF-012
  def test_a_reloaded_node_script_answers_the_default_its_changed_source_exports
    with_node(:exporter).call(:speed?)

    stub_source(:exporter, FASTER) { assert_equal 20, load_script(:exporter).get_property_default_value(:speed) }
  end

  private

  def load_script(name)
    Godot::ResourceLoader.load("#{DIRECTORY}/#{name}.rb")
  end

  def with_node(name)
    node = autofree(Godot::Node.new)
    node.set_script(load_script(name))
    node
  end

  # Runs the block once the script of that name has reloaded holding
  # `source`, then reloads it with what it held.
  def stub_source(name, source)
    held = load_script(name).source_code
    reload(name, source)
    yield
  ensure
    reload(name, held)
  end

  def reload(name, source)
    load_script(name).source_code = source
    load_script(name).reload(true)
    wait_process_frames 1
  end
end
