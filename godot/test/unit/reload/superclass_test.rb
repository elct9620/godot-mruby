class SuperclassTest < Minitest::Test
  DIRECTORY = "res://test/unit/reload".freeze

  CANVAS_CHAMELEON = <<~RUBY.freeze
    module Unit
      module Reload
        class Chameleon < Godot::Node2D
          export :hue, 1

          def kind
            self.class.superclass.to_s
          end
        end
      end
    end
  RUBY

  HALF_CHAMELEON = <<~RUBY.freeze
    module Unit
      module Reload
        class Chameleon < Godot::Node2D
          raise "Chameleon is not finished"
        end
      end
    end
  RUBY

  SPATIAL_CHAMELEON = <<~RUBY.freeze
    module Unit
      module Reload
        class Chameleon < Godot::Node3D
          export :hue, 1
        end
      end
    end
  RUBY

  CANVAS_LARVA = <<~RUBY.freeze
    module Unit
      module Reload
        class Larva < Godot::Node2D
        end
      end
    end
  RUBY

  # @behavior RF-014
  def test_a_node_script_whose_changed_source_names_another_superclass_takes_it
    node = make_node(:chameleon)
    node.call(:kind)

    stub_source(:chameleon, CANVAS_CHAMELEON) { assert_equal "Godot::Node2D", node.call(:kind) }
  end

  # @behavior RF-015
  def test_a_node_whose_class_took_another_superclass_keeps_its_exported_values
    node = make_node(:chameleon)
    node.call(:paint)

    stub_source(:chameleon, CANVAS_CHAMELEON) { assert_equal ["Godot::Node2D", 7], [node.call(:kind), node.get(:hue)] }
  end

  # @behavior RF-016
  def test_a_class_inheriting_a_class_that_took_another_superclass_is_made_afresh_on_it
    node = make_node(:moth)
    node.call(:lineage)

    stub_source(:larva, CANVAS_LARVA) do
      assert_equal "Unit::Reload::Larva < Godot::Node2D", node.call(:lineage)
    end
  end

  # @behavior RF-017
  def test_a_changed_source_naming_another_superclass_that_raises_leaves_the_class_as_it_was
    node = make_node(:chameleon)
    node.call(:kind)

    stub_source(:chameleon, HALF_CHAMELEON) do
      assert_equal ["node", Godot::Node], [node.call(:kind), Unit::Reload::Chameleon.superclass]
    end
  end

  # @behavior RF-018
  def test_a_node_whose_class_took_a_superclass_its_own_class_does_not_reach_loses_its_script
    node = make_node(:chameleon)
    node.call(:kind)

    stub_source(:chameleon, SPATIAL_CHAMELEON) { assert_nil node.get_script }
  end

  # @behavior RF-019
  def test_a_node_that_lost_its_script_to_a_superclass_gets_it_back_with_its_values
    node = make_node(:chameleon)
    node.call(:paint)

    lost = stub_source(:chameleon, SPATIAL_CHAMELEON) { node.get_script.nil? }

    assert_equal [true, 7], [lost, node.get(:hue)]
  end

  private

  def load_script(name)
    Godot::ResourceLoader.load("#{DIRECTORY}/#{name}.rb")
  end

  # A node of a class both the scripts' engine classes reach but Node3D.
  def make_node(name)
    node = autofree(Godot::Node2D.new)
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
    script = load_script(name)
    script.source_code = source
    script.reload(true)
    wait_process_frames 1
  end
end
