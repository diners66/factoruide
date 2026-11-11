-- Quests of the guide the game does not have, or that their giver does not offer: the guide
-- passes them, and the app leaves them out of the next guides.
--
-- 1. The server is asked about the quests the guide still has to take
--    (C_QuestLog.RequestLoadQuestByID, answered by QUEST_DATA_LOAD_RESULT): a quest it does not
--    know twice is missing (a quest of another version of the game the data still lists:
--    Mending Old Wounds on WoW Forever).
-- 2. A quest window opening at the giver of a current accept step without that quest is "not
--    offered", unless a turn-in there may still unlock it or the character is below its level.
--
-- Kept per character for every guide: FactoruideDB.characters[key].unavailable[questID] =
-- { why = "missing" | "not offered", npc, level, time }. The app reads it with the profile.

local FG = Factoruide
local T = FG.T
local QuestLog = FG.QuestLog
local Availability = {}
FG.Availability = Availability

local PER_SECOND = 10 -- server requests a second
local ASKS = 2 -- "no" answers that make a quest missing

local store -- this character's unavailable quests
local queue, asked, noes = {}, {}, {} -- quests to ask, asked and not answered, "no" answers
local pumping = false
local lastList = nil -- { entity, time } of the last quest list window (gossip, greeting)

local L = {
    missing = T("%s is not in the game: passed"),
    notOffered = T("%s is not offered by %s: passed"),
}

--- Why the game does not let the character have this quest (nil: nothing known against it).
function Availability:Why(questID)
    local entry = store and store[questID]
    return entry and entry.why
end

--- Whether the guide passes this quest: unavailable, and neither taken nor done.
function Availability:Passed(questID)
    return questID ~= nil
        and self:Why(questID) ~= nil
        and not QuestLog.IsOnQuest(questID)
        and not QuestLog.IsQuestFlaggedCompleted(questID)
end

--- Name of a quest in the guide (its accept step's text), for the chat.
local function QuestName(questID)
    local route = FG.Guide:Route()
    for _, step in ipairs(route and route.steps or {}) do
        if step.q == questID and step.k == "accept" then
            return step.n and ((step.t or ""):gsub("^Accept ", ""):gsub(" from .*$", "")) or step.t
        end
    end
    return "#" .. questID
end

local function Mark(questID, why, npcName, npcID)
    if not store or store[questID] then return end
    store[questID] = { why = why, npc = npcID, level = UnitLevel("player"), time = time() }
    if why == "missing" then
        FG:Print(L.missing, QuestName(questID))
    else
        FG:Print(L.notOffered, QuestName(questID), npcName or "?")
    end
    FG.Guide:Advance()
end

-- 1. The server ------------------------------------------------------------------------------

local function CanAsk() return C_QuestLog and C_QuestLog.RequestLoadQuestByID ~= nil end

local function Pump()
    pumping = false
    local sent = 0
    while sent < PER_SECOND and #queue > 0 do
        local q = table.remove(queue, 1)
        asked[q] = true
        C_QuestLog.RequestLoadQuestByID(q)
        sent = sent + 1
    end
    if #queue > 0 then
        pumping = true
        C_Timer.After(1, Pump)
    end
end

local function Ask(questID)
    if asked[questID] then return end
    table.insert(queue, questID)
    if not pumping then
        pumping = true
        C_Timer.After(1, Pump)
    end
end

--- Ask the server about the quests the guide followed still has to take.
function Availability:Check()
    local route = FG.Guide:Route()
    if not (store and route and CanAsk()) then return end
    local seen = {}
    for _, step in ipairs(route.steps) do
        local q = step.q
        if
            q
            and step.k == "accept"
            and not seen[q]
            -- Those found missing too: a "yes" now (a server answer lost at login, a new version
            -- of the game) gives them back.
            and not QuestLog.IsOnQuest(q)
            and not QuestLog.IsQuestFlaggedCompleted(q)
        then
            seen[q] = true
            Ask(q)
        end
    end
end

FG:On("QUEST_DATA_LOAD_RESULT", function(questID, success)
    -- Only our own requests: other addons ask too.
    if not asked[questID] then return end
    asked[questID] = nil
    if success then
        noes[questID] = nil
        -- Known after all: a former "missing" goes.
        if store and store[questID] and store[questID].why == "missing" then store[questID] = nil end
        return
    end
    noes[questID] = (noes[questID] or 0) + 1
    if noes[questID] >= ASKS then
        Mark(questID, "missing")
    else
        Ask(questID) -- once more, a moment later
    end
end)

-- 2. The giver -------------------------------------------------------------------------------

local function Giver()
    local entityType, entityID = FG.ParseGUID(UnitGUID("npc"))
    return entityType, entityID, UnitName("npc")
end

-- Steps ahead of the current one looked at for this giver's quests.
local AHEAD = 20

--- Whether a quest of level `questLevel` is gray for the character (the planner's
--- `xp::Rules::gray_level`): its giver may not list it (low-level quests not tracked).
local function Gray(questLevel)
    if not questLevel then return false end
    local level = UnitLevel("player")
    local gray
    if level <= 5 then
        gray = 0
    elseif level <= 39 then
        gray = level - 5 - math.floor(level / 10)
    elseif level <= 59 then
        gray = level - 1 - math.floor(level / 5)
    else
        gray = level - 9
    end
    return questLevel <= gray
end

--- Whether the quests an accept step needs done are (`pre`: all of them, `pa`: `pn` of them,
--- one when not said).
local function PrerequisitesDone(step)
    for _, q in ipairs(step.pre or {}) do
        if not QuestLog.IsQuestFlaggedCompleted(q) then return false end
    end
    if step.pa and #step.pa > 0 then
        local done = 0
        for _, q in ipairs(step.pa) do
            if QuestLog.IsQuestFlaggedCompleted(q) then done = done + 1 end
        end
        if done < (step.pn or 1) then return false end
    end
    return true
end

--- Accept steps at this giver, from the current step on (a few ahead), whose quest is neither
--- offered nor in the log nor done. Those after a quest to turn in there first are not judged
--- (the turn-in may unlock them), nor those after a turn-in or an accept at another NPC still
--- to do, nor any when the window lists a quest to turn in, nor those above the character's
--- level, gray for it, or whose prerequisites are not done.
local function NotOffered(entityType, entityID, offered, turnins)
    local route = FG.Guide:Route()
    if not route or turnins then return {} end
    local out = {}
    local from = FG.Guide:CurrentStep()
    for index = from, math.min(from + AHEAD, #route.steps) do
        local step = route.steps[index]
        if
            step.k == "turnin"
            and step.e == entityType
            and step.id == entityID
            and not QuestLog.IsQuestFlaggedCompleted(step.q)
        then
            return {}
        end
        -- A turn-in elsewhere, or a quest taken elsewhere, may unlock the accepts after it (a
        -- prerequisite, a quest to hold): those are judged at a later visit, not now.
        if
            (step.k == "turnin" or step.k == "accept")
            and not (step.e == entityType and step.id == entityID)
            and step.q
            and not QuestLog.IsQuestFlaggedCompleted(step.q)
            and not (step.k == "accept" and QuestLog.IsOnQuest(step.q))
        then
            break
        end
        if
            step.k == "accept"
            and step.e == entityType
            and step.id == entityID
            and not offered[step.q]
            and not QuestLog.IsOnQuest(step.q)
            and not QuestLog.IsQuestFlaggedCompleted(step.q)
            and UnitLevel("player") >= math.max(step.lvl or 0, step.min or 0)
            and not Gray(step.ql)
            and PrerequisitesDone(step)
        then
            table.insert(out, step.q)
        end
    end
    return out
end

local function Judge(offered, turnins)
    local entityType, entityID, name = Giver()
    if not (store and entityType and entityID) then return end
    -- Offered now: a quest once not offered is again (a step done since, a level reached).
    for q in pairs(offered) do
        if store[q] and store[q].why == "not offered" then store[q] = nil end
    end
    for _, q in ipairs(NotOffered(entityType, entityID, offered, turnins)) do
        Mark(q, "not offered", name, entityID)
    end
end

FG:On("GOSSIP_SHOW", function()
    local entityType, entityID = Giver()
    lastList = { entity = entityID, time = GetTime() }
    local offered = {}
    for _, q in ipairs(C_GossipInfo.GetAvailableQuests()) do
        offered[q.questID] = true
    end
    local active = C_GossipInfo.GetActiveQuests()
    if entityType then Judge(offered, #active > 0) end
end)

FG:On("QUEST_GREETING", function()
    local entityType, entityID = Giver()
    lastList = { entity = entityID, time = GetTime() }
    local offered = {}
    for i = 1, GetNumAvailableQuests() do
        local questID = select(5, GetAvailableQuestInfo(i))
        -- Older clients do not say which quest: nothing judged then.
        if not questID or questID == 0 then return end
        offered[questID] = true
    end
    if entityType then Judge(offered, GetNumActiveQuests() > 0) end
end)

-- A giver with one quest and nothing to say shows it at once: that quest is all it offers.
FG:On("QUEST_DETAIL", function(questStartItemID)
    if questStartItemID and questStartItemID ~= 0 then return end
    local _, entityID = Giver()
    -- Chosen from the list just shown: judged there.
    if lastList and lastList.entity == entityID and GetTime() - lastList.time < 30 then return end
    local questID = GetQuestID()
    if not questID or questID == 0 then return end
    Judge({ [questID] = true }, false)
end)

-- The list is not forgotten when its window closes: choosing a quest in it closes the gossip
-- before the quest's text opens (QUEST_DETAIL), which would then judge the giver's other quests.

-- A new level may unlock a quest that was not offered: judged again at the next visit. Those
-- the server does not know stay (asked again at each login).
FG:On("PLAYER_LEVEL_UP", function()
    if not store then return end
    for q, entry in pairs(store) do
        if entry.why == "not offered" then store[q] = nil end
    end
end)

-- Taken after all (another giver, a later version of the game): no longer unavailable.
FG:On("QUEST_ACCEPTED", function(arg1, arg2)
    local questID = arg2 or arg1
    if store and questID then store[questID] = nil end
end)

FG:On("PLAYER_LOGIN", function()
    local char = FactoruideDB.characters[FG:CharacterKey()]
    char.unavailable = char.unavailable or {}
    store = char.unavailable
    C_Timer.After(5, function() FG:Try("availability", Availability.Check, Availability) end)
end)

--- Another guide chosen: its quests asked about too.
function Availability:GuideChanged() self:Check() end
