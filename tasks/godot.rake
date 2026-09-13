# frozen_string_literal: true

require_relative "support/godot"

namespace :godot do
  desc "Load the addon in a headless editor, then run the smoke scene and check what its Ruby printed"
  task :verify do
    Godot.verify!
  end
end
