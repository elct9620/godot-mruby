# frozen_string_literal: true

require_relative "support/godot"

namespace :godot do
  desc "Load the addon headless, then check its scenes and the Ruby tests"
  task :verify do
    Godot.verify!
  end
end

desc "Open the editor on godot/ with a fresh host build"
task editor: "extension:build" do
  sh Godot::EXECUTABLE, "--editor", "--path", Godot::PROJECT
end
