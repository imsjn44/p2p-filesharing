import { TextInput, ActionIcon, useMantineTheme, Skeleton } from '@mantine/core';
import { ChevronRightIcon, MagnifyingGlassIcon, ChevronLeftIcon } from '@modulz/radix-icons';
// import { invoke } from '@tauri-apps/api/tauri';
import { useNavigate } from 'react-router-dom';
import { useContext, useState } from 'react';
import { MyContext } from './Context';
import { emit, listen } from '@tauri-apps/api/event';
import { appWindow } from "@tauri-apps/api/window";

// const {invoke} = window.__TAURI__.event; 
export function SearhTab(props) {
  const { clickSearch, setClickSearch, setSearchResults } = useContext(MyContext);

  const navigate = useNavigate();

  const theme = useMantineTheme();

  const [searchString, setSearchString] = useState("");
  const handleChange = (event) => {
    setSearchString(event.target.value);
  };
  const handleClick = async () => {
    setSearchResults([])
    navigate('remotefiles');
    const unlisten = await listen('search_response', (event) => {
      const data = event.payload;
      console.log(data);
      setSearchResults(data);
    });
    await emit('search_query', searchString);
    unlisten();


  };
  return (
    <TextInput
      value={searchString}
      onChange={handleChange}
      icon={<MagnifyingGlassIcon size="1.1rem" stroke={1.5} />}
      radius="md"
      size="md"

      rightSection={
        <ActionIcon size={32} radius="xl" color={theme.primaryColor} variant="filled" onClick={handleClick}>
          {theme.dir === 'ltr' ? (
            <ChevronRightIcon size="1.1rem" stroke={1.5} />
          ) : (
            <ChevronLeftIcon size="1.1rem" stroke={1.5} />
          )}
        </ActionIcon>
      }
      placeholder="Search"
      rightSectionWidth={42}
      {...props}
    />
  );
}